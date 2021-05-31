use gemini_fetch::*;
use lazy_static::*;
use regex::{Regex, RegexSet};
use std::{convert::TryInto, io::Write};
use tokio::*;
use url::form_urlencoded::Parse;

type StrResult<T> = Result<T, &'static str>;
/// Structures for representing the page buffer and history.
// TODO: Add more types!
enum GemTextLine {
    H1(String),
    H2(String),
    H3(String),
    Link(usize, String, url::Url),
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

/// Enum representing all of the available commands and their associated data.
enum ParseResponse {
    GoUrl(url::Url),
    SearchBackwards(String),
    SearchForwards(String),
    FollowLink(usize), // Index of link on page.
    JumpToLine(isize),
    GoBack,
    GoForward,
    Print {
        use_range: bool,
        start: usize,
        stop: usize,
    },
    Enumerate(u32, u32), // Range to enumerate.
    Page(u32),           // Number of lines to page.
    History(u32),        // Number of entries to show.
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
        match prompt(&buf) {
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
fn prompt(buf: &PageBuf) -> StrResult<ParseResponse> {
    print!("*");
    let _ = std::io::stdout().flush();
    let mut response = String::new();
    let _bytes_read = std::io::stdin().read_line(&mut response).unwrap();
    return parse_response(response, buf);
}

// Parse the users command.
// Called by prompt to match input to commands.
fn parse_response(resp: String, buf: &PageBuf) -> StrResult<ParseResponse> {
    lazy_static! {
            static ref NUM_REGEX : regex::Regex = Regex::new(r"^(\d+)\s*$").unwrap();                    // Number only
            static ref NUM_LETTER_REGEX : regex::Regex = Regex::new(r"^(\d+)([a-z]+)\s*$").unwrap();     // Number and letter
            static ref RANGE_LETTER : regex::Regex = Regex::new(r"^(\d+),(\d+)([a-z]+)\s*$").unwrap();    // Range and letter
            static ref LETTER_REGEX : regex::Regex = Regex::new(r"^([a-z]+)\s*$").unwrap();              // Letter only
            static ref LETTER_ARG_REGEX : regex::Regex = Regex::new(r"^([a-z])\s([^\s]+)\s*$").unwrap(); // Letter and arg
    }

    if resp == "\n" {
        return Ok(ParseResponse::Empty);
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
                let num = num.as_str().parse::<usize>().unwrap();
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
        if let Some(cmds) = RANGE_LETTER.captures(&resp) {
            if let (Some(num_start), Some(num_end), Some(cmd)) =
                (cmds.get(1), cmds.get(2), cmds.get(3))
            {
                // TODO: Make sure our number parsing works here.
                let num_start = num_start.as_str().parse::<usize>().unwrap();
                let num_end = num_end.as_str().parse::<usize>().unwrap();
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
                        use_range: true,
                        start: buf.curr_line as usize,
                        stop: buf.curr_line as usize,
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
                    "f" => parse_link_command(arg),
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

fn parse_link_command(id: &str) -> StrResult<ParseResponse> {
    if let Ok(id) = id.parse::<usize>() {
        return Ok(ParseResponse::FollowLink(id));
    }
    Err("Invalid link id.")
}

/// Command Implementations and Helpers
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
        ParseResponse::FollowLink(dest_id) => {
            for line in &buf.lines {
                if let GemTextLine::Link(id, _, url) = line {
                    if *id == dest_id {
                        match go_url(&url).await {
                            Ok(page) => {
                                if page.body.is_some() {
                                    if let Ok(_) = load_page(&page, buf, hist) {
                                        println!("{}", page.body.unwrap().len());
                                    }
                                }
                                return true;
                            },
                            Err(msg) => println!("{}", msg),
                        }
                    }
                }
            }
            return true;

        },
        ParseResponse::Quit => return false,
        ParseResponse::Empty => {
            let cmd = ParseResponse::Print {
                use_range: false,
                start: 0,
                stop: 0,
            };
            if let Ok(val) = print_with_args(&cmd, buf) {
                return true;
            }
        }

        ParseResponse::Invalid => println!("?"),
        _ => println!("NOT YET IMPLEMENTED"),
    }
    return true;
}

// Attempt to fetch a page.
async fn go_url(url: &url::Url) -> StrResult<Page> {
    if let Ok(page) = gemini_fetch::Page::fetch_and_handle_redirects(&url).await {
        return Ok(page);
    }
    return Err("Unable to fetch url.");
}

// Print part of the page (no line numbers)
fn print_with_args(cmd: &ParseResponse, buf: &mut PageBuf) -> StrResult<bool> {
    return match cmd {
        ParseResponse::Print {
            use_range,
            start,
            stop,
        } => {
            if !use_range {
                let start: usize = buf.curr_line;
                if let Some(line) = buf.lines.get(start) {
                    match line {
                        GemTextLine::H1(str) => println!("{}", str),
                        GemTextLine::H2(str) => println!("{}", str),
                        GemTextLine::H3(str) => println!("{}", str),
                        GemTextLine::Line(str) => println!("{}", str),
                        GemTextLine::Link(id, text, _) => println!("[{}] => {}", id, text),
                        _ => return Ok(false),
                    }
                    buf.curr_line += 1;
                    return Ok(true);
                }
                return Ok(false);
            } else {
                let start = *start;
                let stop = *stop;
                let print_range = start..=stop;
                for i in print_range {
                    if let Some(line) = buf.lines.get(i) {
                        match line {
                            GemTextLine::H1(str) => println!("{}", str),
                            GemTextLine::H2(str) => println!("{}", str),
                            GemTextLine::H3(str) => println!("{}", str),
                            GemTextLine::Line(str) => println!("{}", str),
                            GemTextLine::Link(id, text, _) => println!("[{}] => {}", id, text),
                            _ => return Ok(false),
                        }
                    }
                }
                buf.curr_line = stop;
                return Ok(true);
            }
            println!("NO PRINTING YET");
            Ok(true)
        }
        _ => Err("BAD THINGS HAPPENED"),
    };
}

/// Functions for representing the page buffer and history.
fn load_page(raw: &gemini_fetch::Page, buf: &mut PageBuf, hist: &mut History) -> StrResult<bool> {
    if raw.header.meta.starts_with("text/gemini") {
        if let Some(body) = &raw.body {
            buf.lines.clear();
            let mut link_count: usize = 0;
            let mut lines = body.split("\n");
            buf.curr_line = 0;
            while let Some(line) = lines.next() {
                if line.starts_with("#") {
                    if let Ok(parsed) = parse_gemtext_header(line) {
                        buf.lines.push(parsed);
                    }
                } else if line.starts_with("=>") {
                    if let Ok(parsed) = parse_gemtext_link(line, &mut link_count) {
                        buf.lines.push(parsed);
                    } else {
                        println!("Unable to parse link: {}", line);
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
    } else {
        println!("NOT GEMINI: {}", raw.url.as_str());
        println!("{}", raw.header.meta);
        return Err("Unable to load page!");
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

fn parse_gemtext_link(line: &str, id: &mut usize) -> StrResult<GemTextLine> {
    lazy_static! {
        static ref WHITESPACE_ONLY: regex::Regex = Regex::new(r"^\s*$").unwrap();
        static ref LINK_REGEX: regex::Regex = Regex::new(r"^=>\s+([^\s]+)\s+(.+)$").unwrap();
        static ref URL_REGEX: regex::Regex = Regex::new(r"^=>\s+([^\s]+)\s*$").unwrap();
    }

    if LINK_REGEX.is_match(line) {
        if let Some(captures) = LINK_REGEX.captures(line) {
            if let (Some(url_str), Some(text)) = (captures.get(1), captures.get(2)) {
                if let Ok(parsed_url) = url::Url::parse(url_str.as_str()) {
                    *id = *id + 1;
                    return Ok(GemTextLine::Link(
                        *id,
                        text.as_str().to_string(),
                        parsed_url,
                    ));
                }
            }
        }
    } else if URL_REGEX.is_match(line) {
        if let Some(captures) = URL_REGEX.captures(line) {
            if let Some(url_str) = captures.get(1) {
                if let Ok(parsed_url) = url::Url::parse(url_str.as_str()) {
                    *id = *id + 1;
                    return Ok(GemTextLine::Link(
                        *id,
                        url_str.as_str().to_string(),
                        parsed_url,
                    ));
                }
            }
        }
    }

    Err("Unable to parse link.")
}
