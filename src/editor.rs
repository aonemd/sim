use crate::Document;
use crate::Row;
use crate::Terminal;

use termion::event::Key;
use termion::color;
use std::env;
use std::time::{Duration, Instant};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const STATUS_BG_COLOR: color::Rgb = color::Rgb(239, 239, 239);
const STATUS_FG_COLOR: color::Rgb = color::Rgb(63, 63, 63);
const QUIT_TIMES: u8 = 3;

#[derive(PartialEq, Copy, Clone)]
pub enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Default, Clone)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

struct StatusMessage {
    text: String,
    time: Instant,
}

impl StatusMessage {
    fn from(message: String) -> Self {
        Self {
            time: Instant::now(),
            text: message,
        }
    }
}

pub struct Editor {
    should_quit: bool,
    terminal: Terminal,
    document: Document,
    cursor_position: Position,
    // will keep track of what row of the file the user is currently scrolled to
    offset: Position,
    status_message: StatusMessage,
    quit_times: u8,
}

impl Editor {
    pub fn default() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut initial_status = String::from("HELP: Ctrl-Q = quit | Ctrl-Q = quit | Ctrl-F = find");
        let document = if let Some(file_name) = args.get(1) {
            let doc = Document::open(&file_name);
            // if doc.is_ok() {
            //     doc.unwrap()
            // } else {
            //     initial_status = format!("ERR: Could not open file: {}", file_name);
            //     Document::default()
            // }
            if let Ok(unwrapped_doc) = doc {
                unwrapped_doc
            } else {
                initial_status = format!("ERR: Could not open file: {}", file_name);
                Document::default()
            }
        } else {
            Document::default()
        };

        Self {
            should_quit: false,
            terminal: Terminal::default().expect("Failed to initialize terminal"),
            document,
            cursor_position: Position::default(),
            offset: Position::default(),
            status_message: StatusMessage::from(initial_status),
            quit_times: QUIT_TIMES,
        }
    }

    pub fn run(&mut self) {
        loop {
            if let Err(error) = self.refresh_screen() {
                die(error);
            }

            if self.should_quit {
                break;
            }

            if let Err(error) = self.process_keypress() {
                die(error);
            }
        }
    }

    fn refresh_screen(&self) -> Result<(), std::io::Error> {
        Terminal::cursor_hide();
        Terminal::cursor_position(&Position::default());

        if self.should_quit {
            Terminal::clear_screen();
            println!("Goodbye\r");
        } else {
            self.draw_rows();

            self.draw_status_bar();
            self.draw_message_bar();

            // Terminal::cursor_position(&self.cursor_position);
            Terminal::cursor_position(&Position {
                x: self.cursor_position.x.saturating_sub(self.offset.x),
                y: self.cursor_position.y.saturating_sub(self.offset.y),
            });
        }

        Terminal::cursor_show();
        Terminal::flush()
    }

    fn search(&mut self) {
        let old_position = self.cursor_position.clone();
        let mut direction = SearchDirection::Forward;
        let query = self
            .prompt("Search (ESC to cancel, Arrows to navigate): ", |editor, key, query| {
                let mut moved =false;
                match key {
                    Key::Right | Key::Down => {
                        direction = SearchDirection::Forward;
                        editor.move_cursor(Key::Right);
                        moved = true;
                    },
                    Key::Left | Key::Up => direction = SearchDirection::Backward,
                    _ => direction = SearchDirection::Forward,
                }

                if let Some(position) = editor.document.find(&query, &editor.cursor_position, direction) {
                    editor.cursor_position = position;
                    editor.scroll();
                } else if moved {
                    editor.move_cursor(Key::Left);
                }
                editor.document.highlight(Some(query));
            }).unwrap_or(None);

        if query.is_none() {
            self.cursor_position = old_position;
            self.scroll();
        }
        self.document.highlight(None);
    }

    #[allow(clippy::integer_arithmetic)]
    fn process_keypress(&mut self) -> Result<(), std::io::Error> {
        let pressed_key = Terminal::read_key()?;
        match pressed_key {
            Key::Ctrl('q') => {
                if self.quit_times > 0 && self.document.is_dirty() {
                    self.status_message = StatusMessage::from(format!(
                            "WARNING! File has unsaved changes. Press Ctrl-Q {} more times to quit.",
                            self.quit_times,
                    ));

                    self.quit_times -= 1;
                    return Ok(());
                }

                self.should_quit = true;
            },
            Key::Ctrl('s') => self.save(),
            Key::Ctrl('f') => self.search(),
            Key::Char(c) => {
                self.document.insert(&self.cursor_position, c);
                self.move_cursor(Key::Right);
            },
            Key::Delete => self.document.delete(&self.cursor_position),
            Key::Backspace => {
                if self.cursor_position.x > 0 || self.cursor_position.y > 0 {
                    self.move_cursor(Key::Left);
                    self.document.delete(&self.cursor_position);
                }
            },
            Key::Up
            | Key::Down
            | Key::Left
            | Key::Right
            | Key::PageUp
            | Key::PageDown
            | Key::End
            | Key::Home => {
                self.move_cursor(pressed_key);
            },
            _ => (),
        }

        self.scroll();

        if self.quit_times < QUIT_TIMES {
            self.quit_times = QUIT_TIMES;
            self.status_message = StatusMessage::from(String::new());
        }

        Ok(())
    }

    fn save(&mut self) {
        if self.document.file_name.is_none() {
            let new_name = self.prompt("Save as: ", |_, _, _| {}).unwrap_or(None);
            if new_name.is_none() {
                self.status_message = StatusMessage::from("Save aborted.".to_string());
                return;
            }

            self.document.file_name = new_name;
        }

        if self.document.save().is_ok() {
            self.status_message = StatusMessage::from("File saved successfully.".to_string());
        } else {
            self.status_message = StatusMessage::from("Error writing to file!".to_string());
        }
    }

    fn prompt<C>(&mut self, prompt: &str, mut callback: C) -> Result<Option<String>, std::io::Error>
    where
        C: FnMut(&mut Self, Key, &String),
        {
            let mut result = String::new();
            loop {
                self.status_message = StatusMessage::from(format!("{}{}", prompt, result));
                self.refresh_screen()?;
                let key = Terminal::read_key()?;
                match key {
                    Key::Backspace => {
                        result.truncate(result.len().saturating_sub(1));
                    },
                    Key::Char('\n') => break,
                    Key::Char(c) => {
                        if !c.is_control() {
                            result.push(c);
                        }
                    }
                    Key::Esc => {
                        result.truncate(0);
                        break;
                    }
                    _ => (),
                }

                callback(self, key, &result);
            }

            self.status_message = StatusMessage::from(String::new());
            if result.is_empty() {
                return Ok(None);
            }

            Ok(Some(result))
        }

    fn scroll(&mut self) {
        let Position { x, y } = self.cursor_position;
        let width = self.terminal.size().width as usize;
        let height = self.terminal.size().height as usize;
        let mut offset = &mut self.offset;

        if y < offset.y {
            offset.y = y;
        } else if y >= offset.y.saturating_add(height) {
            // check if the cursor has moved outside of the visible window, and if so, adjust
            // offset so that the cursor is just inside the visible window
            offset.y = y.saturating_sub(height).saturating_add(1);
        }

        if x < offset.x {
            offset.x = x;
        } else if x >= offset.x.saturating_add(width) {
            // check if the cursor has moved outside of the visible window, and if so, adjust
            // offset so that the cursor is just inside the visible window
            offset.x = x.saturating_sub(width).saturating_add(1);
        }
    }

    fn move_cursor(&mut self, key: Key) {
        let terminal_height = self.terminal.size().height as usize;
        let Position { mut y, mut x } = self.cursor_position;
        let height = self.document.len();
        // Now let’s fix the horizontal scrolling. The missing piece here is that we are not yet
        // allowing the cursor to scroll past the right of the screen
        // let width = if let Some(row) = self.document.row(y) {
        //     row.len()
        // } else {
        //     0
        // };
        // let width = self.document.row(y).map_or(0, |row| row.len());
        let mut width = self.document.row(y).map_or(0, Row::len);

        match key {
            Key::Up => y = y.saturating_sub(1),
            Key::Down => {
                if y < height {
                    y = y.saturating_add(1)
                }
            }
            Key::Left => {
                if x > 0 {
                    x = x.saturating_sub(1)
                } else if y > 0 {
                    // We want to allow the user to press at the beginning of the line to move to
                    // the end of the previous line.
                    y -= 1;
                    x = self.document.row(y).map_or(0, Row::len);
                }
            },
            Key::Right => {
                if x < width {
                    x = x.saturating_add(1)
                } else if y < height {
                    // Similarly, let’s allow the user to press at the end of a line to go to the
                    // beginning of the next line.
                    y += 1;
                    x = 0;
                }
            }
            // let’s make the and keys scroll up or down an entire page instead of the full
            // document.
            Key::PageUp => {
                y = if y > terminal_height {
                    y.saturating_sub(terminal_height)
                } else {
                    0
                }
            },
            // let’s make the and keys scroll up or down an entire page instead of the full
            // document.
            Key::PageDown => {
                // We were able to get rid of unnecessary saturating arithmetics. Why? For example, y and height
                // have the same type. If y.saturating_add(terminal_height) is less than height, then y +
                // terminal_height is also less than height.
                y = if y.saturating_add(terminal_height) < height {
                    y.saturating_add(terminal_height)
                } else {
                    height
                }
            },
            Key::Home => x = 0,
            Key::End => x = width,
            _ => (),
        }

        // We have to set width again, since row can have changed during the key processing
        width = self.document.row(y).map_or(0, Row::len);
        // We then set the new value of x, making sure that x does not exceed the current row’s
        // width
        if x > width {
            x = width;
        }

        self.cursor_position = Position { x, y };
    }

    pub fn draw_row(&self, row: &Row) {
        let width = self.terminal.size().width as usize;
        let start = self.offset.x;
        let end = self.offset.x.saturating_add(width);
        let row = row.render(start, end);

        println!("{}\r", row);
    }

    #[allow(clippy::integer_arithmetic, clippy::integer_division)]
    fn draw_rows(&self) {
        let height = self.terminal.size().height;
        for terminal_row in 0..height {
            Terminal::clear_current_line();

            if let Some(row) = self.document.row(self.offset.y.saturating_add(terminal_row as usize)) {
                self.draw_row(row);
            } else if self.document.is_empty() && terminal_row == height / 3 {
                self.draw_welcome_message();
            } else {
                println!("~\r");
            }
        }
    }


    fn draw_status_bar(&self) {
        let mut status;
        let width = self.terminal.size().width as usize;
        let mut file_name = "[No Name]".to_string();
        if let Some(name) = &self.document.file_name {
            file_name = name.clone();
            file_name.truncate(20);
        }

        let modified_indicator = if self.document.is_dirty() {
            " [+]"
        } else {
            ""
        };

        status = format!(
            "{} - {} lines{}",
            file_name,
            self.document.len(),
            modified_indicator
        );

        let line_indicator = format!(
            "{} | {}/{}",
            self.document.file_type(),
            self.cursor_position.y.saturating_add(1),
            self.document.len(),
        );

        #[allow(clippy::integer_arithmetic)]
        let len = status.len() + line_indicator.len();

        // fill the rest of the statusbar with empty spaces
        status.push_str(&" ".repeat(width.saturating_sub(len)));

        status = format!("{}{}", status, line_indicator);
        status.truncate(width);


        Terminal::set_fg_color(STATUS_FG_COLOR);
        Terminal::set_bg_color(STATUS_BG_COLOR);
        println!("{}\r", status);

        Terminal::reset_fg_color();
        Terminal::reset_bg_color();
    }

    fn draw_message_bar(&self) {
        Terminal::clear_current_line();
        let message = &self.status_message;
        if Instant::now() - message.time < Duration::new(5, 0) {
            let mut text = message.text.clone();
            text.truncate(self.terminal.size().width as usize);
            print!("{}", text);
        }
    }

    fn draw_welcome_message(&self) {
        // width/2 - welcome_len/2
        // (width - welcome_len) / 2
        let mut welcome_message = format!("SIM editor -- version {}", VERSION);
        let width = self.terminal.size().width as usize;
        let len = welcome_message.len();
        #[allow(clippy::integer_arithmetic, clippy::integer_division)]
        let padding = width.saturating_sub(len) / 2;
        let spaces = " ".repeat(padding.saturating_sub(1));
        welcome_message = format!("~{}{}", spaces, welcome_message);
        welcome_message.truncate(width);
        println!("{}\r", welcome_message);
    }
}

fn die(e: std::io::Error) {
    Terminal::clear_screen();
    panic!(e);
}

// fn to_ctrl_byte(c: char) -> u8 {
//     let byte = c as u8;
//     byte & 0b0001_1111
// }
