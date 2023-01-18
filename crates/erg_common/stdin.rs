use crate::traits::IN_BLOCK;
use std::cell::RefCell;
use std::process::Command;
use std::thread::LocalKey;
use std::process::Output;

use crossterm::{
    cursor::{CursorShape, MoveToColumn, SetCursorShape},
    event::{read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::Print,
    terminal::{Clear, ClearType},
};

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
    history_input_position: usize,
}
impl StdinReader {
    #[cfg(target_os = "linux")]
    fn access_clipboard() -> Option<Output> {
        if let Ok(str) = std::fs::read("/proc/sys/kernel/osrelease") {
            let str = std::str::from_utf8(&str).unwrap();
            if str.to_ascii_lowercase().contains("microsoft") {
                return Some(Command::new("powershell")
                    .args(["get-clipboard"])
                    .output()
                    .expect("failed to get clipboard"));
            }
        }
        None
    }
    #[cfg(target_os = "macos")]
    fn access_clipboard() -> Option<Output>  {
        Some(Command::new("pbpast")
            .output()
            .expect("failed to get clipboard"))
    }
    
    #[cfg(target_os = "windows")]
    fn access_clipboard() -> Option<Output> {
        Some(Command::new("powershell")
            .args(["get-clipboard"])
            .output()
            .expect("failed to get clipboard"))
    }

    pub fn read(&mut self) -> String {
        crossterm::terminal::enable_raw_mode().unwrap();
        let mut output = std::io::stdout();
        execute!(output, SetCursorShape(CursorShape::Line)).unwrap();
        let mut line = String::new();
        let mut position = 0;
        let mut consult_history = false;
        while let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = read().unwrap()
        {
            consult_history = false;
            match (code, modifiers) {
                (KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                    execute!(output, Print("\n".to_string())).unwrap();
                    return ":exit".to_string();
                }
                (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
                    let op = Self::access_clipboard();
                    let output = match op {
                        None => {continue;}
                        Some(output) => {output}
                    };
                    let clipboard = {
                        let this = String::from_utf8_lossy(&output.stdout).to_string();
                        this.trim_matches(|c: char| c.is_whitespace())
                            .to_string()
                            .replace(['\n', '\r'], "")
                    };
                    line.insert_str(position, &clipboard);
                    position += clipboard.len();
                }
                (.., KeyModifiers::CONTROL) => {
                    continue;
                }
                (KeyCode::Home, ..) => {
                    position = 0;
                }
                (KeyCode::End, ..) => {
                    position = line.len();
                }
                (KeyCode::Backspace, ..) => {
                    if position == 0 {
                        continue;
                    }
                    line.remove(position - 1);
                    position -= 1;
                }
                (KeyCode::Delete, ..) => {
                    if position == line.len() {
                        continue;
                    }
                    line.remove(position);
                }
                (KeyCode::Enter, ..) => {
                    break;
                }
                (KeyCode::Up, ..) => {
                    consult_history = true;
                    if self.history_input_position == 0 {
                        continue;
                    }
                    self.history_input_position -= 1;
                    line = self.buf[self.history_input_position].clone();
                    position = line.len();
                }
                (KeyCode::Down, ..) => {
                    if self.history_input_position == self.buf.len() {
                        continue;
                    }
                    if self.history_input_position == self.buf.len() - 1 {
                        line = "".to_string();
                        position = 0;
                        self.history_input_position += 1;
                        print!("{}\r", Clear(ClearType::CurrentLine));
                        unsafe {
                            if IN_BLOCK {
                                execute!(output, Print("... ".to_owned())).unwrap();
                            } else {
                                execute!(output, Print(">>> ".to_owned())).unwrap();
                            }
                        }
                        continue;
                    }
                    self.history_input_position += 1;
                    line = self.buf[self.history_input_position].clone();
                    position = line.len();
                }
                (KeyCode::Left, ..) => {
                    if position == 0 {
                        continue;
                    }
                    position -= 1;
                }
                (KeyCode::Right, ..) => {
                    if position == line.len() {
                        continue;
                    }
                    position += 1;
                }
                (KeyCode::Char(c), ..) => {
                    line.insert(position, c);
                    position += 1;
                }
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
            execute!(output, MoveToColumn(position as u16 + 4)).unwrap();
        }
        crossterm::terminal::disable_raw_mode().unwrap();
        let buf = {
            let this = &line;
            this.trim_matches(|c: char| c.is_whitespace()).to_string()
        };
        if !consult_history {
            self.history_input_position = self.buf.len() + 1;
        }
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
    static READER: RefCell<StdinReader> = RefCell::new(StdinReader{ lineno: 0, buf: vec![], history_input_position: 0});
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
