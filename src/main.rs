use gemini_fetch::*;
use url::*;
use tokio::*;
use std::io::Write;

#[tokio::main]
async fn main() {
    println!("Rei: A Line Mode Gemini Browser");
    let mut cont = true;
    while cont {
        if let Ok(p) = prompt() {
            if(execute_command(p).await) {
                continue;
            } 
        }     
        cont = false;
    }
}

// Functions for handling pages and such.
async fn get_page(url : &str) -> Result<gemini_fetch::Page, String> {
    if let Ok(url) = url::Url::parse(url) {
        if let Ok(page) = gemini_fetch::Page::fetch(&url, None).await {
            return Ok(page);
        }
    }
    Err("Unable to load url!".to_string())
}

async fn page_test() {
    if let Ok(page) = get_page("gemini://flounder.online/").await {
        println!("PAGE FETCHED");
        let body = page.body;
        if let Some(body) = body {
            println!("{}", body);
        }
    } else {
        println!("OH NOES!");
    }
    println!("EXITING.");
}

// Functions for user interaction.
fn prompt() -> Result<ParseResponse, String> {
    print!("* ");
    let _ = std::io::stdout().flush();
    let mut response = String::new();
    let bytes_read = std::io::stdin().read_line(&mut response).unwrap();
    return  parse_response(response);
}

enum ParseResponse {
    Document(String),
    GoUrl(url::Url),
    SearchBackwards(String),
    SearchForwards(String),
    GoBack,
    GoForward,
    Print(u32, u32),
    Enumerate(u32, u32),
    Page(u32),
    History(u32),
    Quit,
}
fn parse_response(resp : String) -> Result<ParseResponse, String> {
    if resp.len() < 2 {
       return Err("SHORT RESPONSE".to_string()); 
    }

    let mut tokens = resp.split(" ");
    let cmd = tokens.next();
    match cmd {
        Some("g") => {
            if let Some(url) = tokens.next() {
                if let Ok(url) = url::Url::parse(url) {
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

async fn execute_command(cmd : ParseResponse) -> bool {
    match cmd {
        ParseResponse::GoUrl(url) => {
            if let Ok(page) = gemini_fetch::Page::fetch(&url, None).await {
                if let Some(body) = page.body {
                    println!("{}", body);
                }
            }
        },
        ParseResponse::Quit => return false,
        _ => println!("NOT YET IMPLEMENTED"),
    }
    return true;
}
