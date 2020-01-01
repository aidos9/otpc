use crate::item::Item;
use crate::item_storage;
use clipboard::{ClipboardContext, ClipboardProvider};
use std::io;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvTimeoutError;
use std::thread;
use std::time::Duration;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use tui::backend::TermionBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Paragraph, SelectableList, Text, Widget};
use tui::Terminal;

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
            self.draw_menu()?;

            // We use a timeout because otherwise the loop runs too fast and consumes alot of the CPU, this means w still receive input instantly but
            // also can update the codes every second.
            match receiver.recv_timeout(Duration::from_secs(1)) {
                Ok(c) => match c {
                    Ok(k) => match k {
                        Key::Ctrl(c) => {
                            if c == 'c' {
                                return Ok(());
                            }
                        }
                        Key::Char(c) => {
                            if c == 'q' {
                                self.quit();
                            } else if c == 'c' {
                                self.copy();
                            } else if c == 'r' {
                                self.remove();
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
                    Err(_) => return Err("Could not read user input."),
                },
                Err(RecvTimeoutError::Timeout) => (),
                Err(RecvTimeoutError::Disconnected) => {
                    return Err("Could not connect to the input thread.")
                }
            }
        }
    }

    fn draw_menu(&mut self) -> Result<(), &'static str> {
        match &self.current_menu {
            TermMenu::Main => return self.draw_main_menu(self.selected_index),
            _ => return self.draw_main_menu(self.selected_index),
        }
    }

    fn draw_main_menu(&mut self, selected_index: usize) -> Result<(), &'static str> {
        let mut items: Vec<String> = Vec::new();

        for item in self.items.clone() {
            let code_string: String;
            match item.get_code() {
                Ok(code) => code_string = code,
                Err(_) => code_string = String::from("Error"),
            }

            items.push(format!("{} - {}", item.label, code_string));
        }

        let copy_status = self.copy_status.clone();

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

            let text = vec![
                Text::raw("n - New      "),
                Text::raw("e - Edit      "),
                copy_text,
                Text::raw("r - Delete      "),
                Text::raw("q - Quit"),
            ];

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

    fn spawn_stdin_channel() -> Receiver<Result<termion::event::Key, std::io::Error>> {
        let (tx, rx) = mpsc::channel::<Result<termion::event::Key, std::io::Error>>();
        thread::spawn(move || loop {
            for c in io::stdin().keys() {
                let _ = tx.send(c);
            }
        });

        return rx;
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

    fn remove(&mut self) {
        if self.current_menu == TermMenu::Main {
        } else {
            return;
        }
    }

    fn reset_changing_fields(&mut self) {
        self.copy_status = Status::None;
        self.alternate_footer = String::new();
    }
}
