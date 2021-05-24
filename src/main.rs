use gemini_fetch::*;
use url::*;
use tokio::*;
use std::io::Write;

#[tokio::main]
async fn main() {
    println!("Rei: A Line Mode Gemini Browser");
    let mut cont = true;
    let mut buf = PageBuf {
        raw : String::new(),
        lines : Vec::new(),
        curr_line : 0,
        url : None,
    };
    let mut hist = History {
        entry : Vec::new(),
        curr_entry : 0,
    };
    while cont {
        if let Ok(p) = prompt() {
            if execute_command(p, &mut buf, &mut hist).await {
                continue;
            } 
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
    let bytes_read = std::io::stdin().read_line(&mut response).unwrap();
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
                if let Ok(mut url) = url::Url::parse(url) {
                    if !(url.scheme() == "gemini") {
                        return Err("Not gemini://...".to_string());
                    }
                    return Ok(ParseResponse::GoUrl(url));
                }
            }
        },
        Some("q") => return Ok(ParseResponse::Quit),
        _ => return Err("Unknown command.".to_string()),
    }
    println!("OH NO YOU GOOFED");

    return Err("Unable to parse response.".to_string());
}

// Execute the users passed in command.
async fn execute_command(cmd : ParseResponse, buf : &mut PageBuf, hist : &mut History) -> bool {
    match cmd {
        ParseResponse::GoUrl(url) => { 
            if let Ok(found) = go_url(url, buf, hist).await {
                return true; //TODO: If we got false, do something?
            }
            println!("PAGE FETCH ERROR");
        },
        ParseResponse::Quit => return false,
        _ => println!("NOT YET IMPLEMENTED"),
    }
    return true;
}

/// Command Implementations
// Follow the passed in url
async fn go_url(url : url::Url, buf : &mut PageBuf, hist : &mut History) -> Result<bool, String> {
    if let Ok(page) = gemini_fetch::Page::fetch(&url, None).await {
        println!("{:?}", page);
        if let Some(body) = page.body {
            println!("{}", body);
            return Ok(true);
        }
    }
    return Err("DANGER".to_string());
}

/// Structures/functions for representing the current page buffer.
// TODO
struct PageBuf {
    raw : String,
    lines : Vec<String>,
    curr_line : u32,
    url : Option<url::Url>,
}

/// Structures/functions for representing history.
// TODO
struct History {
    entry : Vec<url::Url>,
    curr_entry : u32,
}

