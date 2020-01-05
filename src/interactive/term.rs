use crate::item::{Digits, Item};
use crate::item_storage;
use crate::util::*;
use clipboard::{ClipboardContext, ClipboardProvider};
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
use tui::widgets::{Block, Borders, Paragraph, SelectableList, Text, Widget};
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
    new_item_label: Option<String>,
    new_item_secret: Option<String>,
    new_item_digits: Option<String>,
    new_item_period: Option<String>,
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
            new_item_label: None,
            new_item_secret: None,
            new_item_digits: None,
            new_item_period: None,
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
            TermMenu::New => {
                self.new_item_label = None;
                self.new_item_secret = None;
                self.new_item_digits = None;
                self.new_item_period = None;
                let _ = self.terminal.hide_cursor();
                self.field_cursor_x = 0;
            }
            _ => (),
        }

        match new_menu {
            TermMenu::Edit => {
                self.editing_item_index = Some(self.selected_index);
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
            _ => return self.draw_main_menu(),
        }
    }

    fn new_menu(
        &mut self,
        receiver: &Receiver<Result<termion::event::Key, std::io::Error>>,
    ) -> Result<(), &'static str> {
        self.draw_new_menu()?;
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

        match Term::get_key(receiver)? {
            Some(k) => match k {
                Key::Char(c) => {
                    self.reset_changing_fields();

                    if !c.is_whitespace() {
                        if self.selected_index == 0 {
                            self.new_item_label
                                .get_or_insert(String::new())
                                .insert(self.field_cursor_x as usize, c);
                            self.field_cursor_x += 1;
                        } else if self.selected_index == 1 {
                            if is_base_32_c(c) {
                                self.new_item_secret
                                    .get_or_insert(String::new())
                                    .insert(self.field_cursor_x as usize, c);
                                self.field_cursor_x += 1;
                            }
                        } else if self.selected_index == 2 {
                            if c == '6' || c == '7' || c == '8' {
                                self.new_item_digits
                                    .get_or_insert(String::new())
                                    .insert(self.field_cursor_x as usize, c);
                                self.field_cursor_x += 1;
                            }
                        } else if self.selected_index == 3 {
                            if c.is_numeric() {
                                self.new_item_period
                                    .get_or_insert(String::new())
                                    .insert(self.field_cursor_x as usize, c);
                                self.field_cursor_x += 1;
                            }
                        }
                    } else if c == '\n' {
                        self.reset_changing_fields();

                        if self.selected_index < 3 {
                            self.selected_index += 1;
                            self.new_item_menu_check_x();
                        } else {
                            if self.new_menu_add_item() {
                                self.save()?;
                                self.switch_menu(TermMenu::Main);
                            }
                        }
                    }
                }
                Key::Backspace => {
                    self.reset_changing_fields();

                    if self.selected_index == 0 {
                        let str = self.new_item_label.get_or_insert(String::new());
                        if str.len() > 0 && self.field_cursor_x < (str.len() as u16 + 1) {
                            str.remove((self.field_cursor_x as usize) - 1);
                            self.field_cursor_x -= 1;
                        }
                    } else if self.selected_index == 1 {
                        let str = self.new_item_secret.get_or_insert(String::new());
                        if str.len() > 0 && self.field_cursor_x < (str.len() as u16 + 1) {
                            str.remove((self.field_cursor_x as usize) - 1);
                            self.field_cursor_x -= 1;
                        }
                    } else if self.selected_index == 2 {
                        let str = self.new_item_digits.get_or_insert(String::new());
                        if str.len() > 0 && self.field_cursor_x < (str.len() as u16 + 1) {
                            str.remove((self.field_cursor_x as usize) - 1);
                            self.field_cursor_x -= 1;
                        }
                    } else if self.selected_index == 3 {
                        let str = self.new_item_period.get_or_insert(String::new());
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

                    self.new_item_menu_check_x();
                }
                Key::Down => {
                    self.reset_changing_fields();

                    if self.selected_index == 3 {
                        self.selected_index = 0;
                    } else {
                        self.selected_index += 1;
                    }

                    self.new_item_menu_check_x();
                }
                Key::Right => {
                    self.reset_changing_fields();

                    if self.selected_index == 0 {
                        match &self.new_item_label {
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
                        match &self.new_item_secret {
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
                        match &self.new_item_digits {
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
                        match &self.new_item_period {
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
                        match &self.new_item_label {
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
                        match &self.new_item_secret {
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
                        match &self.new_item_digits {
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
                        match &self.new_item_period {
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

    fn draw_new_menu(&mut self) -> Result<(), &'static str> {
        let label_input;

        match self.new_item_label {
            Some(ref s) => label_input = vec![Text::raw(s)],
            None => {
                label_input = vec![Text::raw(String::new())];
                self.new_item_label = Some(String::new());
            }
        }

        let secret_input;
        match self.new_item_secret {
            Some(ref s) => secret_input = vec![Text::raw(s)],
            None => {
                secret_input = vec![Text::raw(String::new())];
                self.new_item_secret = Some(String::new());
            }
        }

        let digits_input;
        match self.new_item_digits {
            Some(ref s) => digits_input = vec![Text::raw(s)],
            None => {
                digits_input = vec![Text::raw(String::new())];
                self.new_item_digits = Some(String::new());
            }
        }

        let period_input;
        match self.new_item_period {
            Some(ref s) => period_input = vec![Text::raw(s)],
            None => {
                period_input = vec![Text::raw(String::new())];
                self.new_item_period = Some(String::new());
            }
        }

        let selected_index = self.selected_index;
        let alternate_footer = &self.alternate_footer;

        match self.terminal.draw(|mut f| {
            let root_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(93), Constraint::Percentage(7)].as_ref())
                .split(f.size());

            // Wrapping block.
            Block::default()
                .borders(Borders::ALL)
                .title("New")
                .render(&mut f, root_chunks[0]);

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

            Paragraph::new(label_input.iter())
                .block(Block::default().borders(Borders::ALL).title("Label"))
                .alignment(Alignment::Left)
                .render(&mut f, vert_chunks[0]);
            Paragraph::new(secret_input.iter())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Secret (Base-32)"),
                )
                .alignment(Alignment::Left)
                .render(&mut f, vert_chunks[1]);
            Paragraph::new(digits_input.iter())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Digits (6/7/8)"),
                )
                .alignment(Alignment::Left)
                .render(&mut f, vert_chunks[2]);
            Paragraph::new(period_input.iter())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Period (seconds)"),
                )
                .alignment(Alignment::Left)
                .render(&mut f, vert_chunks[3]);

            let mut text;

            if alternate_footer.is_empty() {
                text = vec![Text::raw("Esc - Back")];

                if selected_index == 3 {
                    text.insert(0, Text::raw("Enter - Add      "));
                } else {
                    text.insert(0, Text::raw("Enter - Next      "));
                }
            } else {
                text = vec![Text::raw(alternate_footer)];
            }

            Paragraph::new(text.iter())
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center)
                .render(&mut f, root_chunks[1]);
        }) {
            Ok(_) => (),
            Err(_) => return Err("Could not draw the new item menu"),
        }

        return Ok(());
    }

    fn new_menu_add_item(&mut self) -> bool {
        let label: String;
        let secret: String;
        let digits: Digits;
        let period: u32;

        // Check that all fields have been filled out with valid types.
        match &self.new_item_label {
            Some(s) => {
                if s.is_empty() {
                    self.alternate_footer = String::from("A label is required.");
                    return false;
                }

                if contains_white_space(&String::from(s.trim())) {
                    self.alternate_footer =
                        String::from("No whitespace is permitted in the label.");
                    return false;
                }

                if contains_item_label(&s, &self.items) {
                    self.alternate_footer = String::from("An item with this label already exists.");
                    return false;
                }

                label = s.clone();
            }
            None => {
                self.alternate_footer = String::from("A label is required.");
                return false;
            }
        }

        match &self.new_item_secret {
            Some(s) => {
                if s.is_empty() {
                    self.alternate_footer = String::from("A valid base-32 secret is required.");
                    return false;
                }

                if !is_base_32(s) {
                    self.alternate_footer = String::from("A valid base-32 secret is required.");
                    return false;
                }

                secret = s.clone();
            }
            None => {
                self.alternate_footer = String::from("A valid base-32 secret is required.");
                return false;
            }
        }

        match &self.new_item_digits {
            Some(s) => {
                if s.is_empty() {
                    self.alternate_footer = String::from("A valid number of digits is required.");
                    return false;
                }

                if s == "6" {
                    digits = Digits::Six;
                } else if s == "7" {
                    digits = Digits::Seven;
                } else if s == "8" {
                    digits = Digits::Eight;
                } else {
                    self.alternate_footer = String::from("A valid number of digits is required.");
                    return false;
                }
            }
            None => {
                self.alternate_footer = String::from("A valid number of digits is required.");
                return false;
            }
        }

        match &self.new_item_period {
            Some(s) => {
                if s.is_empty() {
                    self.alternate_footer = String::from("A valid period is required.");
                    return false;
                }

                if !is_number(&s) {
                    self.alternate_footer = String::from("A valid period is required.");
                    return false;
                }

                match s.parse::<u32>() {
                    Ok(n) => period = n,
                    Err(_) => {
                        self.alternate_footer = String::from("A valid period is required.");
                        return false;
                    }
                }

                if period == 0 {
                    self.alternate_footer =
                        String::from("A valid period greater than 0 is required.");
                    return false;
                }
            }
            None => {
                self.alternate_footer = String::from("A valid period is required.");
                return false;
            }
        }

        self.items.push(Item {
            label,
            secret,
            digits,
            split_time: period,
        });

        return true;
    }

    fn new_item_menu_check_x(&mut self) {
        if self.selected_index == 0 {
            match &self.new_item_label {
                Some(s) => {
                    if self.field_cursor_x > s.width() as u16 {
                        self.field_cursor_x = s.width() as u16;
                    }
                }
                None => self.field_cursor_x = 0,
            }
        } else if self.selected_index == 1 {
            match &self.new_item_secret {
                Some(s) => {
                    if self.field_cursor_x > s.width() as u16 {
                        self.field_cursor_x = s.width() as u16;
                    }
                }
                None => self.field_cursor_x = 0,
            }
        } else if self.selected_index == 2 {
            match &self.new_item_digits {
                Some(s) => {
                    if self.field_cursor_x > s.width() as u16 {
                        self.field_cursor_x = s.width() as u16;
                    }
                }
                None => self.field_cursor_x = 0,
            }
        } else if self.selected_index == 3 {
            match &self.new_item_period {
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
                        }
                    }else {
                        if c == 'y' {
                            self.remove()?;
                        }else {
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
        let mut items: Vec<String> = Vec::new();

        for item in self.items.clone() {
            let code_string: String;
            match item.get_code() {
                Ok(code) => code_string = code,
                Err(_) => code_string = String::from("Error"), // Simple announcement because we don't want a long description overflowing the display.
            }

            items.push(format!("{} - {}", item.label, code_string));
        }

        let copy_status = self.copy_status.clone();
        let selected_index = self.selected_index.clone();
        let alternate_footer = self.alternate_footer.clone();

        match self.terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(93), Constraint::Percentage(7)].as_ref())
                .split(f.size());

            let style = Style::default();

            SelectableList::default()
                .block(
                    Block::default()
                        .borders(Borders::TOP | Borders::RIGHT | Borders::LEFT | Borders::BOTTOM)
                        .title("Main Menu"),
                )
                .items(&items)
                .select(Some(selected_index))
                .style(style)
                .highlight_style(style.fg(Color::Magenta).modifier(Modifier::BOLD))
                .render(&mut f, chunks[0]);

            let text;

            if alternate_footer.is_empty() {
                let copy_text;
                match copy_status {
                    Status::None => {
                        copy_text = Text::raw("c - Copy      ");
                    }
                    Status::Success => {
                        copy_text = Text::styled("c - Copy      ", Style::default().fg(Color::Green));
                    }
                    Status::Fail => {
                        copy_text = Text::styled("c - Copy      ", Style::default().fg(Color::Red));
                    }
                }

                text = vec![
                    Text::raw("n - New      "),
                    Text::raw("e - Edit      "),
                    copy_text,
                    Text::raw("r - Delete      "),
                    Text::raw("q - Quit"),
                ];
            }else {
                text = vec![Text::raw(alternate_footer)];
            }

            Paragraph::new(text.iter())
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center)
                .render(&mut f, chunks[1]);
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

        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        match ctx.set_contents(code) {
            Ok(_) => self.copy_status = Status::Success,
            Err(_) => self.copy_status = Status::Fail,
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
