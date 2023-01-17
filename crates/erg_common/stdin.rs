use std::cell::RefCell;
use std::thread::LocalKey;
use crate::traits::IN_BLOCK;

use crossterm::event::{read, Event,KeyCode, KeyEvent, KeyModifiers};
use crossterm::{execute, style::Print};
use crossterm::terminal::{Clear, ClearType};

/// e.g.
/// ```erg
/// >>> print! 1
/// >>>
/// >>> while! False, do!:
/// >>>    print! ""
/// >>>
/// ```
/// ↓
///
/// `{ lineno: 5, buf: ["print! 1\n", "\n", "while! False, do!:\n", "print! \"\"\n", "\n"] }`
#[derive(Debug)]
pub struct StdinReader {
    pub lineno: usize,
    buf: Vec<String>,
}

impl StdinReader {
    pub fn read(&mut self) -> String {
        let mut output = std::io::stdout();
        let mut line = String::new();
        loop {
            match read().unwrap() {
                Event::Key(KeyEvent {code: KeyCode::Char('z'),  modifiers: KeyModifiers::CONTROL, ..}) => {
                    execute!(output, Print("\n".to_string())).unwrap();
                    return ":exit".to_string();
                }
                Event::Key(KeyEvent {code: KeyCode::Backspace, ..}) => {
                    line.pop();
                }
                Event::Key(KeyEvent {code: KeyCode::Enter, .. }) => {
                    break;
                }
                Event::Key(KeyEvent {code: KeyCode::Char(c), ..}) => {line.push(c);}
                _ => {}
            }
            print!("{}\r", Clear(ClearType::CurrentLine));
            unsafe {
                if IN_BLOCK {
                    execute!(output, Print("... ".to_owned() + &line)).unwrap();
                } else {
                    execute!(output, Print(">>> ".to_owned() + &line)).unwrap();
                }
            }
        }
        let buf = {
            let this = &line;
            this.trim_matches(|c: char| c.is_whitespace()).to_string()
        };
        self.lineno += 1;
        self.buf.push(buf);
        self.buf.last().cloned().unwrap_or_default()
    }

    pub fn reread(&self) -> String {
        self.buf.last().cloned().unwrap_or_default()
    }

    pub fn reread_lines(&self, ln_begin: usize, ln_end: usize) -> Vec<String> {
        self.buf[ln_begin - 1..=ln_end - 1].to_vec()
    }
}

thread_local! {
    static READER: RefCell<StdinReader> = RefCell::new(StdinReader{ lineno: 0, buf: vec![] });
}

#[derive(Debug)]
pub struct GlobalStdin(LocalKey<RefCell<StdinReader>>);

pub static GLOBAL_STDIN: GlobalStdin = GlobalStdin(READER);

impl GlobalStdin {
    pub fn read(&'static self) -> String {
        self.0.with(|s| s.borrow_mut().read())
    }

    pub fn reread(&'static self) -> String {
        self.0.with(|s| s.borrow().reread())
    }

    pub fn reread_lines(&'static self, ln_begin: usize, ln_end: usize) -> Vec<String> {
        self.0
            .with(|s| s.borrow_mut().reread_lines(ln_begin, ln_end))
    }
}
