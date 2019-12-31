use crate::item::Item;
use crate::item_storage;
use std::io;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use tui::backend::TermionBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Paragraph, SelectableList, Text, Widget};
use tui::Terminal;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;
use std::{thread, time};

enum TermMenu {
    New,
    Edit,
    Main,
    Individual,
    None,
}

pub struct Term {
    terminal: Terminal<TermionBackend<RawTerminal<io::Stdout>>>,
    items: Vec<Item>,
    current_menu: TermMenu,
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
        };
    }

    pub fn start_main_menu(&mut self) -> Result<(), &'static str> {
        match self.terminal.clear() {
            Ok(_) => (),
            Err(_) => return Err("Could not clear the terminal."),
        }

        match self.terminal.hide_cursor() {
            Ok(_) => (),
            Err(_) => return Err("Could not hide the cursor."),
        }

        let mut selected_index = 0;
        let receiver = Term::spawn_stdin_channel();

        loop {
            self.draw_main_menu(selected_index)?;

            match receiver.try_recv() {
                Ok(c) => {
                    match c {
                        Ok(k) => match k {
                            Key::Ctrl(c) => {
                                if c == 'c' {
                                    return Ok(());
                                }
                            }
                            Key::Char(c) => {
                                if c == 'q' {
                                    self.quit();
                                }
                            }
                            Key::Up => {
                                if selected_index != 0 {
                                    selected_index -= 1;
                                } else {
                                    selected_index = self.items.len() - 1;
                                }
                            }
                            Key::Down => {
                                if selected_index == self.items.len() - 1 {
                                    selected_index = 0;
                                } else {
                                    selected_index += 1;
                                }
                            }
                            _ => (),
                        },
                        Err(_) => return Err("Could not read user input."),
                    }
                },
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => return Err("Could not connect to the input thread.")
            }
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
                .highlight_style(style.fg(Color::LightGreen).modifier(Modifier::BOLD))
                .render(&mut f, chunks[0]);

            let text = [
                Text::raw("n - New      "),
                Text::raw("e - Edit      "),
                Text::raw("c - Copy      "),
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
}
