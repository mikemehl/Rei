use std::collections::HashMap;
use gemini_fetch::*;
use lazy_static::*;
use regex::Regex;
use std::{convert::TryInto, io::Write};
mod interface;
mod exec;
mod marks;

pub type Bookmarks = HashMap<char, String>; 
pub type StrResult<T> = Result<T, &'static str>;
/// Structures for representing the page buffer and history.
// TODO: Add more types!
pub enum GemTextLine {
    H1(String),
    H2(String),
    H3(String),
    Link(usize, String, url::Url),
    Line(String),
}
pub struct PageBuf {
    //page: Option<gemini_fetch::Page>, // The raw page response.
    lines: Vec<GemTextLine>, // The parsed lines for display.
    curr_line: usize,
    url: Option<url::Url>,
}

pub struct History {
    entry: Vec<url::Url>,
    curr_entry: usize,
}

impl History {
    pub fn add(self: &mut History, url: &url::Url) {
        if let Ok(url) = url::Url::parse(url.as_str()) {
            self.curr_entry += 1;
            if self.curr_entry >= self.entry.len() {
                self.entry.push(url);
                self.curr_entry = self.entry.len() - 1;
            } else {
                self.entry[self.curr_entry] = url;
                self.entry.truncate(self.curr_entry + 1);
            }
        }
    }
}

/// Enum representing all of the available commands and their associated data.
pub enum ParseResponse {
    GoUrl(url::Url),
    SearchBackwards(String),
    SearchForwards(String),
    FollowLink(usize), // Index of link on page.
    JumpToLine(usize),
    GoBack(usize),
    GoForward(usize),
    Print {
        use_range: bool,
        start: usize,
        stop: usize,
    },
    Enumerate {
        use_range: bool,
        start: usize,
        stop: usize,
    },
    Page(usize),    // Number of lines to page.
    History(isize), // Number of entries to show (-1 means show all)
    Clear,
    Invalid,
    Empty,
    Quit,
}

/// main()
#[tokio::main]
async fn main() {
    println!("Rei: A Line Mode Gemini Browser");
    let mut cont = true;
    let mut buf = PageBuf {
        lines: Vec::new(),
        curr_line: 0,
        url: None,
    };
    let mut hist = History {
        entry: Vec::new(),
        curr_entry: 0,
    };
    while cont {
        match interface::prompt(&buf) {
            Ok(p) => {
                if exec::execute_command(p, &mut buf, &mut hist).await {
                    continue;
                }
            }
            Err(msg) => println!("{}", msg),
        }
        cont = false;
    }
}
