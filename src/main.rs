use gemini_fetch::*;
use url::*;
use tokio::*;
use std::io::Write;
use regex::Regex;

#[tokio::main]
async fn main() {
    println!("Rei: A Line Mode Gemini Browser");
    let mut cont = true;
    let mut buf = PageBuf {
        page : None,
        lines : Vec::new(),
        curr_line : 0,
        url : None,
    };
    let mut hist = History {
        entry : Vec::new(),
        curr_entry : 0,
    };
    while cont {
        match prompt() {
            Ok(p) => {
                if execute_command(p, &mut buf, &mut hist).await {
                    continue;
                } 
            },
            Err(msg) => println!("{}", msg),
        }
        cont = false;
    }
}

/// Functions for user interaction.
// Prompt for input and return the command.
fn prompt() -> Result<ParseResponse, String> {
    print!("*");
    let _ = std::io::stdout().flush();
    let mut response = String::new();
    let _bytes_read = std::io::stdin().read_line(&mut response).unwrap();
    return  parse_response(response);
}

// All of the available commands and their associated data.
enum ParseResponse {
    GoUrl(url::Url),
    SearchBackwards(String),
    SearchForwards(String),
    FollowLink(u32), // Index of link on page.
    GoBack,
    GoForward,
    Print(u32, u32), // Range to print.
    Enumerate(u32, u32), // Range to enumerate.
    Page(u32), // Number of lines to page.
    History(u32), // Number of entries to show.
    Quit,
}

// Called by prompt to match input to commands.
fn parse_response(resp : String) -> Result<ParseResponse, String> {
    if resp.len() < 2 {
       return Err("SHORT RESPONSE".to_string()); 
    }

    let mut tokens = resp.split(" ");
    let cmd = tokens.next();
    match cmd {
        Some("g") => {
            if let Some(url) = tokens.next() {
                return parse_go_command(url);
            }
        },
        Some("q") => return Ok(ParseResponse::Quit),
        _ => return Err("Unknown command.".to_string()),
    }

    return Err("Unable to parse response.".to_string());
}

fn parse_go_command(mut url : &str) -> Result<ParseResponse, String> {
    let scheme_re = Regex::new(r"^gemini://").unwrap();
    let mut new_url = "gemini://".to_string();
    if !scheme_re.is_match(&url) {
        new_url.push_str(url);
        if let Ok(url) = url::Url::parse(&new_url) {
            return Ok(ParseResponse::GoUrl(url));
        } else {
            return Err("Unable to parse URL.".to_string());
        }
    } else {
        if let Ok(url) = url::Url::parse(url) {
            return Ok(ParseResponse::GoUrl(url));
        } else {
            return Err("Unable to parse URL.".to_string());
        }
    }
}

// Execute the users passed in command.
async fn execute_command(cmd : ParseResponse, buf : &mut PageBuf, hist : &mut History) -> bool {
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
                },
                Err(msg) => println!("{}", msg),

            }
        },
        ParseResponse::Quit => return false,
        _ => println!("NOT YET IMPLEMENTED"),
    }
    return true;
}

/// Command Implementations and Helpers
// Attempt to fetch a page.
async fn go_url(url : &url::Url) -> Result<Page, String> {
    if let Ok(page) = gemini_fetch::Page::fetch_and_handle_redirects(&url).await {
        return Ok(page)
    }
    return Err("Unable to fetch url.".to_string());
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
    page : Option<gemini_fetch::Page>, // The raw page response.
    lines : Vec<GemTextLine>, // The parsed lines for display.
    curr_line : usize,
    url : Option<url::Url>,
}

/// Structures/functions for representing history.
// TODO
struct History {
    entry : Vec<url::Url>,
    curr_entry : usize,
}

