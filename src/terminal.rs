use crate::Position;

use std::io::{self, stdout, Write};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::color;

pub struct Size {
    // u16 => unsigned 16 bits: 65,000 max
    pub width: u16,
    pub height: u16,
}

// ASCII codes 32–126 are all printable
// ASCII codes 0–31 are all control characters, and 127 is also a control character Control
// characters are non-printable characters

pub struct Terminal {
    size: Size,
    // _stdout needed to keep terminal in raw mode not in canonical mode
    _stdout: RawTerminal<std::io::Stdout>,
}

impl Terminal {
    pub fn default() -> Result<Self, std::io::Error> {
        let size = termion::terminal_size()?;

        Ok(Self {
            size: Size {
                width: size.0,
                height: size.1.saturating_sub(2),
            },
            _stdout: stdout().into_raw_mode()?,
        })
    }

    pub fn size(&self) -> &Size {
        &self.size
    }

    pub fn clear_screen() {
        // \x1b => Esc: 27,
        // [2J => J: Erase in Display, 2: argument means clear entire screen
        // https://vt100.net/docs/vt100-ug/chapter3.html#ED
        // https://vt100.net/docs/vt100-ug/chapter3.html
        // https://en.wikipedia.org/wiki/VT100
        // print!("\x1b[2J");
        // OR
        print!("{}", termion::clear::All);
    }

    pub fn clear_current_line() {
        // \x1b[K
        print!("{}", termion::clear::CurrentLine);
    }

    pub fn set_bg_color(color: color::Rgb) {
        print!("{}", color::Bg(color));
    }

    pub fn reset_bg_color() {
        print!("{}", color::Bg(color::Reset));
    }

    pub fn set_fg_color(color: color::Rgb) {
        print!("{}", color::Fg(color));
    }

    pub fn reset_fg_color() {
        print!("{}", color::Fg(color::Reset));
    }

    // 0-based in contrast to vt100 which is 1-based
    #[allow(clippy::cast_possible_truncation)]
    pub fn cursor_position(position: &Position) {
        // https://vt100.net/docs/vt100-ug/chapter3.html#CUP
        // print!("\x1b[1;1H");
        // OR

        // `saturating_add()`: attempts to add 1, and if that’s not possible, it just returns the
        // maximum value; no overflow
        let Position{mut x, mut y} = position;
        x = x.saturating_add(1);
        y = y.saturating_add(1);
        let x = x as u16;
        let y = y as u16;
        print!("{}", termion::cursor::Goto(x, y));
    }

    pub fn flush() -> Result<(), std::io::Error> {
        io::stdout().flush()
    }

    pub fn cursor_hide() {
        // \x1b[25h
        // Set Mode
        // http://vt100.net/docs/vt100-ug/chapter3.html#SM
        print!("{}", termion::cursor::Hide);
    }

    pub fn cursor_show() {
        // \x1b[25l
        // Reset Mode
        // http://vt100.net/docs/vt100-ug/chapter3.html#RM
        print!("{}", termion::cursor::Show);
    }

    pub fn read_key() -> Result<Key, std::io::Error> {
        loop {
            if let Some(key) = io::stdin().lock().keys().next() {
                return key;
            }
        }
    }
}
