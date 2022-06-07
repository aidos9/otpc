use crate::item::{Digits, Item};
use crate::item_storage;
use crate::util::*;
use arboard::Clipboard;
use std::io::{self, Write};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvTimeoutError;
use std::thread;
use std::time::Duration;
use termion::cursor::Goto;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use tui::backend::TermionBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use tui::Terminal;
use unicode_width::UnicodeWidthStr;

#[derive(PartialEq)]
enum TermMenu {
    New,
    Edit,
    Main,
    None,
}

#[derive(Clone)]
enum Status {
    None,
    Success,
    Fail,
}

pub struct Term {
    terminal: Terminal<TermionBackend<RawTerminal<io::Stdout>>>,
    items: Vec<Item>,
    current_menu: TermMenu,
    selected_index: usize,
    copy_status: Status,
    alternate_footer: String,
    editing_item_index: Option<usize>,
    item_label: Option<String>,
    item_secret: Option<String>,
    item_digits: Option<String>,
    item_period: Option<String>,
    field_cursor_x: u16,
    pending_confirmation: bool,
}

impl Term {
    pub fn new() -> Term {
        let backend;
        match io::stdout().into_raw_mode() {
            Ok(out) => backend = TermionBackend::new(out),
            Err(_) => {
                eprintln!("Could not set up stdout.");
                std::process::exit(1);
            }
        }

        let terminal;

        match Terminal::new(backend) {
            Ok(t) => terminal = t,
            Err(_) => {
                eprintln!("Could not set up the terminal for interactive mode.");
                std::process::exit(1);
            }
        }

        let items: Vec<Item>;

        if item_storage::storage_location_exists() {
            match item_storage::retrieve_items(&item_storage::storage_location()) {
                Ok(i) => items = i,
                Err(e) => {
                    eprintln!("An error occurred when reading the database: {}", e);
                    std::process::exit(1);
                }
            }
        } else {
            items = Vec::new();
        }

        return Term {
            terminal,
            items,
            current_menu: TermMenu::None,
            selected_index: 0,
            copy_status: Status::None,
            alternate_footer: String::new(),
            editing_item_index: None,
            item_label: None,
            item_secret: None,
            item_digits: None,
            item_period: None,
            field_cursor_x: 0,
            pending_confirmation: false,
        };
    }

    pub fn start(&mut self) -> Result<(), &'static str> {
        self.current_menu = TermMenu::Main;

        match self.terminal.clear() {
            Ok(_) => (),
            Err(_) => return Err("Could not clear the terminal."),
        }

        match self.terminal.hide_cursor() {
            Ok(_) => (),
            Err(_) => return Err("Could not hide the cursor."),
        }

        let receiver = Term::spawn_stdin_channel();

        loop {
            self.draw_menu(&receiver)?;
        }
    }

    fn switch_menu(&mut self, new_menu: TermMenu) {
        self.reset_changing_fields();

        match self.current_menu {
            TermMenu::New | TermMenu::Edit => {
                self.item_label = None;
                self.item_secret = None;
                self.item_digits = None;
                self.item_period = None;
                let _ = self.terminal.hide_cursor();
                self.field_cursor_x = 0;
            }
            _ => (),
        }

        match new_menu {
            TermMenu::Edit => {
                self.editing_item_index = Some(self.selected_index);
                self.item_label = Some(self.items[self.selected_index].label.clone());
                self.item_secret = Some(self.items[self.selected_index].secret.clone());
                self.item_digits = match self.items[self.selected_index].digits {
                    Digits::Eight => Some(String::from("8")),
                    Digits::Seven => Some(String::from("7")),
                    Digits::Six => Some(String::from("6")),
                };
                self.item_period = Some(format!("{}", self.items[self.selected_index].split_time));

                self.selected_index = 0;
                let _ = self.terminal.show_cursor();
            }
            TermMenu::New => {
                self.selected_index = 0;
                self.editing_item_index = None;
                let _ = self.terminal.show_cursor();
            }
            TermMenu::Main => {
                self.selected_index = 0;
            }
            _ => match self.editing_item_index {
                Some(n) => {
                    self.selected_index = n;
                    self.editing_item_index = None;
                }
                None => (),
            },
        }

        self.current_menu = new_menu;
    }

    fn draw_menu(
        &mut self,
        rec: &Receiver<Result<termion::event::Key, std::io::Error>>,
    ) -> Result<(), &'static str> {
        match &self.current_menu {
            TermMenu::Main => return self.main_menu(rec),
            TermMenu::New => return self.new_menu(rec),
            TermMenu::Edit => return self.edit_menu(rec),
            _ => return self.draw_main_menu(),
        }
    }

    fn edit_menu(
        &mut self,
        receiver: &Receiver<Result<termion::event::Key, std::io::Error>>,
    ) -> Result<(), &'static str> {
        self.draw_edit_menu("Enter - Save      ", "Edit")?;
        // Put the cursor back inside the input box
        match write!(
            self.terminal.backend_mut(),
            "{}",
            Goto(
                4 + self.field_cursor_x,
                1 + (3 * (self.selected_index as u16 + 1))
            )
        ) {
            Ok(_) => (),
            Err(_) => return Err("Could not write to stdout."),
        }

        // stdout is buffered, flush it to see the effect immediately.
        io::stdout().flush().ok();

        return self.handle_edit_input(receiver);
    }

    fn new_menu(
        &mut self,
        receiver: &Receiver<Result<termion::event::Key, std::io::Error>>,
    ) -> Result<(), &'static str> {
        self.draw_edit_menu("Enter - Add      ", "New")?;
        // Put the cursor back inside the input box
        match write!(
            self.terminal.backend_mut(),
            "{}",
            Goto(
                4 + self.field_cursor_x,
                1 + (3 * (self.selected_index as u16 + 1))
            )
        ) {
            Ok(_) => (),
            Err(_) => return Err("Could not write to stdout."),
        }

        // stdout is buffered, flush it to see the effect immediately.
        io::stdout().flush().ok();

        return self.handle_edit_input(receiver);
    }

    fn handle_edit_input(
        &mut self,
        receiver: &Receiver<Result<termion::event::Key, std::io::Error>>,
    ) -> Result<(), &'static str> {
        match Term::get_key(receiver)? {
            Some(k) => match k {
                Key::Char(c) => {
                    self.reset_changing_fields();

                    if !c.is_whitespace() {
                        if self.selected_index == 0 {
                            self.item_label
                                .get_or_insert(String::new())
                                .insert(self.field_cursor_x as usize, c);
                            self.field_cursor_x += 1;
                        } else if self.selected_index == 1 {
                            if is_base_32_c(c) {
                                self.item_secret
                                    .get_or_insert(String::new())
                                    .insert(self.field_cursor_x as usize, c);
                                self.field_cursor_x += 1;
                            }
                        } else if self.selected_index == 2 {
                            if c == '6' || c == '7' || c == '8' {
                                self.item_digits
                                    .get_or_insert(String::new())
                                    .insert(self.field_cursor_x as usize, c);
                                self.field_cursor_x += 1;
                            }
                        } else if self.selected_index == 3 {
                            if c.is_numeric() {
                                self.item_period
                                    .get_or_insert(String::new())
                                    .insert(self.field_cursor_x as usize, c);
                                self.field_cursor_x += 1;
                            }
                        }
                    } else if c == '\n' {
                        self.reset_changing_fields();

                        if self.selected_index < 3 {
                            self.selected_index += 1;
                            self.item_menu_check_x();
                        } else {
                            if self.current_menu == TermMenu::New {
                                if self.new_menu_add_item() {
                                    self.save()?;
                                    self.switch_menu(TermMenu::Main);
                                }
                            } else if self.current_menu == TermMenu::Edit {
                                if self.edit_menu_save_item() {
                                    self.save()?;
                                    self.switch_menu(TermMenu::Main);
                                }
                            }
                        }
                    }
                }
                Key::Backspace => {
                    self.reset_changing_fields();

                    if self.selected_index == 0 {
                        let str = self.item_label.get_or_insert(String::new());
                        if str.len() > 0 && self.field_cursor_x < (str.len() as u16 + 1) {
                            str.remove((self.field_cursor_x as usize) - 1);
                            self.field_cursor_x -= 1;
                        }
                    } else if self.selected_index == 1 {
                        let str = self.item_secret.get_or_insert(String::new());
                        if str.len() > 0 && self.field_cursor_x < (str.len() as u16 + 1) {
                            str.remove((self.field_cursor_x as usize) - 1);
                            self.field_cursor_x -= 1;
                        }
                    } else if self.selected_index == 2 {
                        let str = self.item_digits.get_or_insert(String::new());
                        if str.len() > 0 && self.field_cursor_x < (str.len() as u16 + 1) {
                            str.remove((self.field_cursor_x as usize) - 1);
                            self.field_cursor_x -= 1;
                        }
                    } else if self.selected_index == 3 {
                        let str = self.item_period.get_or_insert(String::new());
                        if str.len() > 0 && self.field_cursor_x < (str.len() as u16 + 1) {
                            str.remove((self.field_cursor_x as usize) - 1);
                            self.field_cursor_x -= 1;
                        }
                    }
                }
                Key::Esc => {
                    self.reset_changing_fields();
                    self.switch_menu(TermMenu::Main);
                }
                Key::Up => {
                    self.reset_changing_fields();

                    if self.selected_index != 0 {
                        self.selected_index -= 1;
                    } else {
                        self.selected_index = 3;
                    }

                    self.item_menu_check_x();
                }
                Key::Down => {
                    self.reset_changing_fields();

                    if self.selected_index == 3 {
                        self.selected_index = 0;
                    } else {
                        self.selected_index += 1;
                    }

                    self.item_menu_check_x();
                }
                Key::Right => {
                    self.reset_changing_fields();

                    if self.selected_index == 0 {
                        match &self.item_label {
                            Some(s) => {
                                // <= because we want the cursor to sit one cell past the last character
                                if self.field_cursor_x + 1 <= s.width() as u16 {
                                    self.field_cursor_x += 1;
                                } else {
                                    self.field_cursor_x = 0;
                                }
                            }
                            None => self.field_cursor_x = 0,
                        }
                    } else if self.selected_index == 1 {
                        match &self.item_secret {
                            Some(s) => {
                                // <= because we want the cursor to sit one cell past the last character
                                if self.field_cursor_x + 1 <= s.width() as u16 {
                                    self.field_cursor_x += 1;
                                } else {
                                    self.field_cursor_x = 0;
                                }
                            }
                            None => self.field_cursor_x = 0,
                        }
                    } else if self.selected_index == 2 {
                        match &self.item_digits {
                            Some(s) => {
                                // <= because we want the cursor to sit one cell past the last character
                                if self.field_cursor_x + 1 <= s.width() as u16 {
                                    self.field_cursor_x += 1;
                                } else {
                                    self.field_cursor_x = 0;
                                }
                            }
                            None => self.field_cursor_x = 0,
                        }
                    } else if self.selected_index == 3 {
                        match &self.item_period {
                            Some(s) => {
                                // <= because we want the cursor to sit one cell past the last character
                                if self.field_cursor_x + 1 <= s.width() as u16 {
                                    self.field_cursor_x += 1;
                                } else {
                                    self.field_cursor_x = 0;
                                }
                            }
                            None => self.field_cursor_x = 0,
                        }
                    }
                }
                Key::Left => {
                    self.reset_changing_fields();

                    if self.selected_index == 0 {
                        match &self.item_label {
                            Some(s) => {
                                // Check if greater than 1 because any subtraction to field_cursor_x will cause integer overflow.
                                if self.field_cursor_x > 1 {
                                    self.field_cursor_x -= 1;
                                } else {
                                    self.field_cursor_x = s.width() as u16;
                                }
                            }
                            None => self.field_cursor_x = 0,
                        }
                    } else if self.selected_index == 1 {
                        match &self.item_secret {
                            Some(s) => {
                                // Check if greater than 1 because any subtraction to field_cursor_x will cause integer overflow.
                                if self.field_cursor_x > 1 {
                                    self.field_cursor_x -= 1;
                                } else {
                                    self.field_cursor_x = s.width() as u16;
                                }
                            }
                            None => self.field_cursor_x = 0,
                        }
                    } else if self.selected_index == 2 {
                        match &self.item_digits {
                            Some(s) => {
                                // Check if greater than 1 because any subtraction to field_cursor_x will cause integer overflow.
                                if self.field_cursor_x > 1 {
                                    self.field_cursor_x -= 1;
                                } else {
                                    self.field_cursor_x = s.width() as u16;
                                }
                            }
                            None => self.field_cursor_x = 0,
                        }
                    } else if self.selected_index == 3 {
                        match &self.item_period {
                            Some(s) => {
                                // Check if greater than 1 because any subtraction to field_cursor_x will cause integer overflow.
                                if self.field_cursor_x > 1 {
                                    self.field_cursor_x -= 1;
                                } else {
                                    self.field_cursor_x = s.width() as u16;
                                }
                            }
                            None => self.field_cursor_x = 0,
                        }
                    }
                }
                _ => (),
            },
            None => (),
        }

        return Ok(());
    }

    fn draw_edit_menu(
        &mut self,
        completion_text: &'static str,
        title: &'static str,
    ) -> Result<(), &'static str> {
        let label_input;

        match self.item_label {
            Some(ref s) => label_input = Spans::from(vec![Span::raw(s)]),
            None => {
                label_input = Spans::from(vec![Span::raw(String::new())]);
                self.item_label = Some(String::new());
            }
        }

        let secret_input;
        match self.item_secret {
            Some(ref s) => secret_input = Spans::from(vec![Span::raw(s)]),
            None => {
                secret_input = Spans::from(vec![Span::raw(String::new())]);
                self.item_secret = Some(String::new());
            }
        }

        let digits_input;
        match self.item_digits {
            Some(ref s) => digits_input = Spans::from(vec![Span::raw(s)]),
            None => {
                digits_input = Spans::from(vec![Span::raw(String::new())]);
                self.item_digits = Some(String::new());
            }
        }

        let period_input;
        match self.item_period {
            Some(ref s) => period_input = Spans::from(vec![Span::raw(s)]),
            None => {
                period_input = Spans::from(vec![Span::raw(String::new())]);
                self.item_period = Some(String::new());
            }
        }

        let selected_index = self.selected_index;
        let alternate_footer = &self.alternate_footer;

        match self.terminal.draw(|f| {
            let root_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Max(100), Constraint::Length(3)].as_ref())
                .split(f.size());

            // Wrapping block.
            f.render_widget(
                Block::default().borders(Borders::ALL).title(title),
                root_chunks[0],
            );

            let vert_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Min(0),
                    ]
                    .as_ref(),
                )
                .split(root_chunks[0]);

            f.render_widget(
                Paragraph::new(label_input)
                    .block(Block::default().borders(Borders::ALL).title("Label"))
                    .alignment(Alignment::Left),
                vert_chunks[0],
            );

            f.render_widget(
                Paragraph::new(secret_input)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Secret (Base-32)"),
                    )
                    .alignment(Alignment::Left),
                vert_chunks[1],
            );

            f.render_widget(
                Paragraph::new(digits_input)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Digits (6/7/8)"),
                    )
                    .alignment(Alignment::Left),
                vert_chunks[2],
            );

            f.render_widget(
                Paragraph::new(period_input)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Period (seconds)"),
                    )
                    .alignment(Alignment::Left),
                vert_chunks[3],
            );

            let text = if alternate_footer.is_empty() {
                Spans::from(vec![
                    if selected_index == 3 {
                        Span::raw(completion_text)
                    } else {
                        Span::raw("Enter - Next      ")
                    },
                    Span::raw("Esc - Back"),
                ])
            } else {
                Spans::from(vec![Span::raw(alternate_footer)])
            };

            f.render_widget(
                Paragraph::new(text)
                    .block(Block::default().borders(Borders::ALL))
                    .alignment(Alignment::Center),
                root_chunks[1],
            );
        }) {
            Ok(_) => (),
            Err(_) => return Err("Could not draw the edit item menu"),
        }

        return Ok(());
    }

    fn edit_menu_save_item(&mut self) -> bool {
        match self.item_menu_construct_item(true) {
            Ok(item) => match self.editing_item_index {
                Some(index) => {
                    if index >= self.items.len() {
                        self.alternate_footer =
                            String::from("Internal error obtaining the correct item.");
                        return false;
                    } else {
                        self.items[index] = item;
                        return true;
                    }
                }
                None => {
                    self.alternate_footer =
                        String::from("Internal error determining the correct item.");
                    return false;
                }
            },
            Err(str) => {
                self.alternate_footer = str;
                return false;
            }
        }
    }

    fn new_menu_add_item(&mut self) -> bool {
        match self.item_menu_construct_item(false) {
            Ok(item) => self.items.push(item),
            Err(str) => {
                self.alternate_footer = str;
                return false;
            }
        }

        return true;
    }

    fn item_menu_construct_item(&mut self, allow_same_name: bool) -> Result<Item, String> {
        let label: String;
        let secret: String;
        let digits: Digits;
        let period: u32;

        // Check that all fields have been filled out with valid types.
        match &self.item_label {
            Some(s) => {
                if s.is_empty() {
                    return Err(String::from("A label is required."));
                }

                if contains_white_space(&String::from(s.trim())) {
                    return Err(String::from("No whitespace is permitted in the label."));
                }

                if !allow_same_name {
                    if contains_item_label(&s, &self.items) {
                        return Err(String::from("An item with this label already exists."));
                    }
                }

                label = s.clone();
            }
            None => {
                return Err(String::from("A label is required."));
            }
        }

        match &self.item_secret {
            Some(s) => {
                if s.is_empty() {
                    return Err(String::from("A valid base-32 secret is required."));
                }

                if !is_base_32(s) {
                    return Err(String::from("A valid base-32 secret is required."));
                }

                secret = s.clone();
            }
            None => {
                return Err(String::from("A valid base-32 secret is required."));
            }
        }

        match &self.item_digits {
            Some(s) => {
                if s.is_empty() {
                    return Err(String::from("A valid number of digits is required."));
                }

                if s == "6" {
                    digits = Digits::Six;
                } else if s == "7" {
                    digits = Digits::Seven;
                } else if s == "8" {
                    digits = Digits::Eight;
                } else {
                    return Err(String::from("A valid number of digits is required."));
                }
            }
            None => {
                return Err(String::from("A valid number of digits is required."));
            }
        }

        match &self.item_period {
            Some(s) => {
                if s.is_empty() {
                    return Err(String::from("A valid period is required."));
                }

                if !is_number(&s) {
                    return Err(String::from("A valid period is required."));
                }

                match s.parse::<u32>() {
                    Ok(n) => period = n,
                    Err(_) => {
                        return Err(String::from("A valid period is required."));
                    }
                }

                if period == 0 {
                    return Err(String::from("A valid period greater than 0 is required."));
                }
            }
            None => {
                return Err(String::from("A valid period is required."));
            }
        }

        return Ok(Item {
            label,
            secret,
            digits,
            split_time: period,
        });
    }

    fn item_menu_check_x(&mut self) {
        if self.selected_index == 0 {
            match &self.item_label {
                Some(s) => {
                    if self.field_cursor_x > s.width() as u16 {
                        self.field_cursor_x = s.width() as u16;
                    }
                }
                None => self.field_cursor_x = 0,
            }
        } else if self.selected_index == 1 {
            match &self.item_secret {
                Some(s) => {
                    if self.field_cursor_x > s.width() as u16 {
                        self.field_cursor_x = s.width() as u16;
                    }
                }
                None => self.field_cursor_x = 0,
            }
        } else if self.selected_index == 2 {
            match &self.item_digits {
                Some(s) => {
                    if self.field_cursor_x > s.width() as u16 {
                        self.field_cursor_x = s.width() as u16;
                    }
                }
                None => self.field_cursor_x = 0,
            }
        } else if self.selected_index == 3 {
            match &self.item_period {
                Some(s) => {
                    if self.field_cursor_x > s.width() as u16 {
                        self.field_cursor_x = s.width() as u16;
                    }
                }
                None => self.field_cursor_x = 0,
            }
        }
    }

    fn main_menu(
        &mut self,
        receiver: &Receiver<Result<termion::event::Key, std::io::Error>>,
    ) -> Result<(), &'static str> {
        self.draw_main_menu()?;

        match Term::get_key(receiver)? {
            Some(k) => match k {
                Key::Char(c) => {
                    if !self.pending_confirmation {
                        if c == 'q' {
                            self.quit();
                        } else if c == 'c' {
                            self.copy();
                        } else if c == 'r' {
                            if self.items.len() > 0 {
                                self.alternate_footer = String::from("y - Delete      n - Cancel");
                                self.pending_confirmation = true;
                            }
                        } else if c == 'n' {
                            self.switch_menu(TermMenu::New);
                        } else if c == 'e' {
                            self.switch_menu(TermMenu::Edit);
                        }
                    } else {
                        if c == 'y' {
                            self.remove()?;
                        } else {
                            self.reset_changing_fields();
                        }
                    }
                }
                Key::Up => {
                    self.reset_changing_fields();

                    if self.selected_index != 0 {
                        self.selected_index -= 1;
                    } else {
                        self.selected_index = self.items.len() - 1;
                    }
                }
                Key::Down => {
                    self.reset_changing_fields();

                    if self.selected_index == self.items.len() - 1 {
                        self.selected_index = 0;
                    } else {
                        self.selected_index += 1;
                    }
                }
                _ => (),
            },
            None => (),
        }

        return Ok(());
    }

    fn draw_main_menu(&mut self) -> Result<(), &'static str> {
        let mut items: Vec<ListItem> = Vec::new();

        for item in self.items.clone() {
            let code_string: String;
            match item.get_code() {
                Ok(code) => code_string = code,
                Err(_) => code_string = String::from("Error"), // Simple announcement because we don't want a long description overflowing the display.
            }

            items.push(ListItem::new(format!("{} - {}", item.label, code_string)));
        }

        let copy_status = self.copy_status.clone();
        let selected_index = self.selected_index.clone();
        let alternate_footer = self.alternate_footer.clone();
        let mut current_state = ListState::default();

        current_state.select(Some(selected_index));

        match self.terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Max(100), Constraint::Length(3)].as_ref())
                .split(f.size());

            let style = Style::default();

            f.render_stateful_widget(
                List::new(items)
                    .block(
                        Block::default()
                            .borders(
                                Borders::TOP | Borders::RIGHT | Borders::LEFT | Borders::BOTTOM,
                            )
                            .title("Main Menu"),
                    )
                    .style(style)
                    .highlight_style(style.fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                chunks[0],
                &mut current_state,
            );

            let text;

            if alternate_footer.is_empty() {
                let copy_text;
                match copy_status {
                    Status::None => {
                        copy_text = Span::raw("c - Copy      ");
                    }
                    Status::Success => {
                        copy_text =
                            Span::styled("c - Copy      ", Style::default().fg(Color::Green));
                    }
                    Status::Fail => {
                        copy_text = Span::styled("c - Copy      ", Style::default().fg(Color::Red));
                    }
                }

                text = Spans::from(vec![
                    Span::raw("n - New      "),
                    Span::raw("e - Edit      "),
                    copy_text,
                    Span::raw("r - Delete      "),
                    Span::raw("q - Quit"),
                ]);
            } else {
                text = Spans::from(vec![Span::raw(alternate_footer)]);
            }

            f.render_widget(
                Paragraph::new(text)
                    .block(Block::default().borders(Borders::ALL))
                    .alignment(Alignment::Center),
                chunks[1],
            );
        }) {
            Ok(_) => (),
            Err(_) => return Err("Could not draw the main menu"),
        }

        return Ok(());
    }

    fn copy(&mut self) {
        let code;

        if self.current_menu == TermMenu::Main {
            match self.items[self.selected_index].get_code() {
                Ok(c) => code = c,
                Err(_) => {
                    self.copy_status = Status::Fail;
                    return;
                }
            }
        } else {
            return;
        }

        if let Ok(mut clipboard) = Clipboard::new() {
            match clipboard.set_text(code) {
                Ok(_) => self.copy_status = Status::Success,
                Err(_) => self.copy_status = Status::Fail,
            }
        } else {
            self.copy_status = Status::Fail;
        }
    }

    fn remove(&mut self) -> Result<(), &'static str> {
        if self.current_menu == TermMenu::Main && self.selected_index < self.items.len() {
            self.items.remove(self.selected_index);

            if self.selected_index > 0 {
                self.selected_index -= 1;
            }

            self.save()?;
            self.reset_changing_fields();
        }

        return Ok(());
    }

    fn reset_changing_fields(&mut self) {
        self.copy_status = Status::None;
        self.alternate_footer = String::new();
        self.pending_confirmation = false;
    }

    fn quit(&mut self) {
        let _ = self.save();
        let _ = self.terminal.show_cursor();
        let _ = self.terminal.clear();

        std::process::exit(0);
    }

    fn save(&self) -> Result<(), &'static str> {
        match item_storage::write_items(&item_storage::storage_location(), &self.items) {
            Ok(()) => return Ok(()),
            Err(e) => {
                eprintln!("An error occurred when saving the database: {}", e);
                std::process::exit(1);
            }
        }
    }

    fn get_key(
        receiver: &Receiver<Result<termion::event::Key, std::io::Error>>,
    ) -> Result<Option<termion::event::Key>, &'static str> {
        // We use a timeout because otherwise the loop runs too fast and consumes alot of the CPU,
        // this timeout means we still receive input instantly but also can update the codes every second.
        match receiver.recv_timeout(Duration::from_secs(1)) {
            Ok(c) => match c {
                Ok(k) => {
                    match k {
                        Key::Ctrl(c) => {
                            if c == 'c' {
                                std::process::exit(2);
                            }
                        }
                        _ => (),
                    }

                    return Ok(Some(k));
                }
                Err(_) => return Err("Could not read user input."),
            },
            Err(RecvTimeoutError::Timeout) => (),
            Err(RecvTimeoutError::Disconnected) => {
                return Err("Could not connect to the input thread.")
            }
        }

        return Ok(None);
    }

    fn spawn_stdin_channel() -> Receiver<Result<termion::event::Key, std::io::Error>> {
        let (tx, rx) = mpsc::channel::<Result<termion::event::Key, std::io::Error>>();
        thread::spawn(move || loop {
            for c in io::stdin().keys() {
                let _ = tx.send(c);
            }
        });

        return rx;
    }
}
