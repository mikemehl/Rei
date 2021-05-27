use gemini_fetch::*;
use lazy_static::*;
use regex::{Regex, RegexSet};
use std::io::Write;
use tokio::*;
use url::*;

type StrResult<T> = Result<T, &'static str>;
/// Structures for representing the page buffer and history.
// TODO: Add more types!
enum GemTextLine {
    H1(String),
    H2(String),
    H3(String),
    Link(String, url::Url),
    Line(String),
    Invalid,
}
struct PageBuf {
    page: Option<gemini_fetch::Page>, // The raw page response.
    lines: Vec<GemTextLine>,          // The parsed lines for display.
    curr_line: usize,
    url: Option<url::Url>,
}

struct History {
    entry: Vec<url::Url>,
    curr_entry: usize,
}

#[tokio::main]
async fn main() {
    println!("Rei: A Line Mode Gemini Browser");
    let mut cont = true;
    let mut buf = PageBuf {
        page: None,
        lines: Vec::new(),
        curr_line: 0,
        url: None,
    };
    let mut hist = History {
        entry: Vec::new(),
        curr_entry: 0,
    };
    while cont {
        match prompt() {
            Ok(p) => {
                if execute_command(p, &mut buf, &mut hist).await {
                    continue;
                }
            }
            Err(msg) => println!("{}", msg),
        }
        cont = false;
    }
}

/// Functions for user interaction.
// Prompt for input and return the command.
fn prompt() -> StrResult<ParseResponse> {
    print!("*");
    let _ = std::io::stdout().flush();
    let mut response = String::new();
    let _bytes_read = std::io::stdin().read_line(&mut response).unwrap();
    return parse_response(response);
}

// All of the available commands and their associated data.
enum ParseResponse {
    GoUrl(url::Url),
    SearchBackwards(String),
    SearchForwards(String),
    FollowLink(u32), // Index of link on page.
    JumpToLine(isize),
    GoBack,
    GoForward,
    Print {
        use_range: bool,
        start: isize,
        stop: isize,
    },
    Enumerate(u32, u32), // Range to enumerate.
    Page(u32),           // Number of lines to page.
    History(u32),        // Number of entries to show.
    Invalid,
    Quit,
}

// Parse the users command.
// Called by prompt to match input to commands.
fn parse_response(resp: String) -> StrResult<ParseResponse> {
    lazy_static! {
            static ref NUM_REGEX : regex::Regex = Regex::new(r"^(\d+)\s*$").unwrap();                    // Number only
            static ref NUM_LETTER_REGEX : regex::Regex = Regex::new(r"^(\d+)([a-z]+)\s*$").unwrap();     // Number and letter
            static ref RANGE_LETTER : regex::Regex = Regex::new(r"^(\d+),(\d+)([a-z])\s*$").unwrap();    // Range and letter
            static ref LETTER_REGEX : regex::Regex = Regex::new(r"^([a-z]+)\s*$").unwrap();              // Letter only
            static ref LETTER_ARG_REGEX : regex::Regex = Regex::new(r"^([a-z])\s([^\s]+)\s*$").unwrap(); // Letter and arg
    }

    if NUM_REGEX.is_match(&resp) {
        if let Some(num) = NUM_REGEX.captures(&resp) {
            if let Some(num) = num.get(1) {
                return Ok(ParseResponse::JumpToLine(
                    num.as_str().parse::<isize>().unwrap(),
                ));
            }
        }
    }

    if NUM_LETTER_REGEX.is_match(&resp) {
        if let Some(cmds) = NUM_LETTER_REGEX.captures(&resp) {
            if let (Some(num), Some(cmd)) = (cmds.get(1), cmds.get(2)) {
                let num = num.as_str().parse::<isize>().unwrap();
                let cmd = cmd.as_str();
                return Ok(match cmd {
                    "p" => ParseResponse::Print {
                        use_range: true,
                        start: num,
                        stop: num,
                    },
                    _ => ParseResponse::Invalid,
                });
            }
        }
    }

    if RANGE_LETTER.is_match(&resp) {
        if let Some(cmds) = NUM_LETTER_REGEX.captures(&resp) {
            if let (Some(num_start), Some(num_end), Some(cmd)) =
                (cmds.get(1), cmds.get(2), cmds.get(3))
            {
                let num_start = num_start.as_str().parse::<isize>().unwrap();
                let num_end = num_end.as_str().parse::<isize>().unwrap();
                let cmd = cmd.as_str();
                return Ok(match cmd {
                    "p" => ParseResponse::Print {
                        use_range: true,
                        start: num_start,
                        stop: num_end,
                    },
                    _ => ParseResponse::Invalid,
                });
            }
        }
    }

    if LETTER_REGEX.is_match(&resp) {
        if let Some(cmd) = LETTER_REGEX.captures(&resp) {
            if let Some(cmd) = cmd.get(1) {
                let cmd = cmd.as_str();
                return Ok(match cmd {
                    "p" => ParseResponse::Print {
                        use_range: false,
                        start: 0,
                        stop: 0,
                    },
                    "q" => ParseResponse::Quit,
                    _ => ParseResponse::Invalid,
                });
            }
        }
    }

    if LETTER_ARG_REGEX.is_match(&resp) {
        if let Some(cmd) = LETTER_ARG_REGEX.captures(&resp) {
            if let (Some(cmd), Some(arg)) = (cmd.get(1), cmd.get(2)) {
                let cmd = cmd.as_str();
                let arg = arg.as_str();
                return match cmd {
                    "g" => parse_go_command(arg),
                    _ => Ok(ParseResponse::Invalid),
                };
            }
        }
    }

    return Ok(ParseResponse::Invalid);
}

fn parse_go_command(url: &str) -> StrResult<ParseResponse> {
    let scheme_re = Regex::new(r"^gemini://").unwrap();
    let mut new_url = "gemini://".to_string();
    if !scheme_re.is_match(&url) {
        new_url.push_str(url);
        if let Ok(url) = url::Url::parse(&new_url) {
            return Ok(ParseResponse::GoUrl(url));
        } else {
            return Err("Unable to parse URL.");
        }
    } else {
        if let Ok(url) = url::Url::parse(url) {
            return Ok(ParseResponse::GoUrl(url));
        } else {
            return Err("Unable to parse URL.");
        }
    }
}

// Execute the users passed in command.
// Returns false if the program should terminate.
async fn execute_command(cmd: ParseResponse, buf: &mut PageBuf, hist: &mut History) -> bool {
    match cmd {
        ParseResponse::GoUrl(url) => match go_url(&url).await {
            Ok(page) => {
                if page.body.is_some() {
                    let _ = load_page(&page, buf, hist);
                    println!("{}", page.body.unwrap().len());
                }
                return true;
            }
            Err(msg) => println!("{}", msg),
        },
        ParseResponse::Print {
            use_range,
            start,
            stop,
        } => {
            if let Ok(val) = print_with_args(&cmd, buf) {
                return true;
            }
        }
        ParseResponse::Quit => return false,
        ParseResponse::Invalid => println!("?"),
        _ => println!("NOT YET IMPLEMENTED"),
    }
    return true;
}

/// Command Implementations and Helpers
// Attempt to fetch a page.
async fn go_url(url: &url::Url) -> StrResult<Page> {
    if let Ok(page) = gemini_fetch::Page::fetch_and_handle_redirects(&url).await {
        return Ok(page);
    }
    return Err("Unable to fetch url.");
}

/// Page Display Functions
// TODO
fn print_with_args(cmd: &ParseResponse, buf: &PageBuf) -> StrResult<bool> {
    return match cmd {
        ParseResponse::Print {
            use_range,
            start,
            stop,
        } => {
            println!("NO PRINTING YET");
            Ok(true)
        }
        _ => Err("BAD THINGS HAPPENED"),
    };
}

/// Functions for representing the page buffer and history.
fn load_page(raw: &gemini_fetch::Page, buf: &mut PageBuf, hist: &mut History) -> StrResult<bool> {
    if raw.header.meta == "text/gemini" {
        if let Some(body) = &raw.body {
            buf.lines.clear();
            let mut lines = body.split("\n");
            while let Some(line) = lines.next() {
                if line.starts_with("#") {
                    if let Ok(parsed) = parse_gemtext_header(line) {
                        buf.lines.push(parsed);
                    }
                } else if line.starts_with("=>") {
                    if let Ok(parsed) = parse_gemtext_link(line) {
                        buf.lines.push(parsed);
                    }
                } else if line.starts_with("```") {
                    while let Some(line) = lines.next() {
                        if line.starts_with("```") {
                            break;
                        } else {
                            buf.lines.push(GemTextLine::Line(line.to_string()));
                        }
                    }
                }
            }
        }
    }
    Ok(true)
}

fn parse_gemtext_header(text: &str) -> StrResult<GemTextLine> {
    let mut header_count = 0;
    for c in text.chars() {
        if c == '#' {
            header_count += 1;
        } else if header_count <= 3 {
            return Ok(match header_count {
                // TODO: Strip off header signifiers.
                1 => GemTextLine::H1(text.to_string()),
                2 => GemTextLine::H2(text.to_string()),
                3 => GemTextLine::H3(text.to_string()),
                _ => GemTextLine::Line(text.to_string()),
            });
        } else {
            break;
        }
    }

    Err("Unable to parse header.")
}

fn parse_gemtext_link(line: &str) -> StrResult<GemTextLine> {
    let mut components = line.split_ascii_whitespace();
    components.next();
    if let Some(url_str) = components.next() {
        if let Ok(url) = url::Url::parse(url_str) {
            if let Some(text) = components.next() {
                return Ok(GemTextLine::Link(text.to_string(), url));
            } else {
                return Ok(GemTextLine::Link(url_str.to_string(), url));
            }
        } else {
            return Ok(GemTextLine::Line(components.collect()));
        }
    }
    Err("Unable to parse link.")
}
