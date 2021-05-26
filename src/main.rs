use gemini_fetch::*;
use regex::Regex;
use std::io::Write;
use tokio::*;
use url::*;

type StrResult<T> = Result<T, &'static str>;

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

// Called by prompt to match input to commands.
fn parse_response(resp: String) -> StrResult<ParseResponse> {
    let mut tokens = resp.split(" ");
    let cmd = tokens.next();
    match cmd {
        Some("g") => {
            if let Some(url) = tokens.next() {
                return parse_go_command(url);
            }
        }
        Some("q\n") => return Ok(ParseResponse::Quit),
        Some("p\n") | Some("\n") => {
            return Ok(ParseResponse::Print {
                use_range: false,
                start: 0,
                stop: 0,
            })
        }
        Some(a) => return parse_response_with_range(resp),
        None => return Ok(ParseResponse::Invalid),
    }

    return Err("Unable to parse response.");
}

// If the user specified a range, parse it.
fn parse_response_with_range(resp: String) -> StrResult<ParseResponse> {
    if let Ok(range_extract) = Regex::new(r"(\d+),(\d+)([a-zA-z])") {
        let mut first_num: Option<isize> = None;
        let mut second_num: Option<isize> = None;
        let mut range_walker = range_extract.find_iter(&resp);
        if let Some(first) = range_walker.next() {
            if let Ok(parse_num) = first.as_str().parse::<isize>() {
                first_num = Some(parse_num);
            } else {
                return Err("Unable to parse ranged command (no numbers).");
            }
        }

        if let Some(second) = range_walker.next() {
            if let Ok(parse_num) = second.as_str().parse::<isize>() {
                // Ranged command, try to parse.
                second_num = Some(parse_num);
                if let Some(command) = range_walker.next() {
                    match command.as_str() {
                        "p" => {
                            return Ok(ParseResponse::Print {
                                use_range: true,
                                start: first_num.unwrap(),
                                stop: second_num.unwrap(),
                            })
                        }
                        _ => return Ok(ParseResponse::Invalid),
                    }
                } else {
                    return Ok(ParseResponse::Invalid);
                }
            } else {
                // Single number command, try to parse.
                match second.as_str() {
                    "p" => {
                        return Ok(ParseResponse::Print {
                            use_range: true,
                            start: first_num.unwrap(),
                            stop: first_num.unwrap(),
                        })
                    }
                    _ => return Ok(ParseResponse::Invalid),
                }
            }
        } else {
            if let Some(jump_line) = first_num {
                return Ok(ParseResponse::JumpToLine(jump_line));
            }
        }
    } else {
        return Err("Allocation failure.");
    }
    return Ok(ParseResponse::Invalid);
}

fn parse_go_command(mut url: &str) -> StrResult<ParseResponse> {
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
        ParseResponse::GoUrl(url) => {
            match go_url(&url).await {
                Ok(page) => {
                    // TODO: Load the buf, don't print!
                    //       What if it's not text???
                    if let Some(body) = page.body {
                        println!("{}", body);
                    }
                    return true;
                }
                Err(msg) => println!("{}", msg),
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

/// Structures/functions for representing the current page buffer.
// TODO

// TODO: Add more types!
enum GemTextLine {
    H1(String),
    H2(String),
    H3(String),
    Link(String, url::Url),
}
struct PageBuf {
    page: Option<gemini_fetch::Page>, // The raw page response.
    lines: Vec<GemTextLine>,          // The parsed lines for display.
    curr_line: usize,
    url: Option<url::Url>,
}

/// Structures/functions for representing history.
// TODO
struct History {
    entry: Vec<url::Url>,
    curr_entry: usize,
}
