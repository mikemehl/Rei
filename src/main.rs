use gemini_fetch::*;
use url::*;
use tokio::*;
use std::io::Write;

#[tokio::main]
async fn main() {
    println!("Rei: A Line Mode Gemini Browser");
    let mut cont = true;
    while cont {
        if let Ok(p) = prompt().await {
            continue;
        } else {
            cont = false;
        }
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
async fn prompt() -> Result<ParseResponse, String> {
    print!("* ");
    let _ = std::io::stdout().flush();
    let mut response = String::new();
    let bytes_read = std::io::stdin().read_line(&mut response).unwrap();
    return parse_response(response).await;
}

enum ParseResponse {
    Document(String),
}
async fn parse_response(resp : String) -> Result<ParseResponse, String> {
    if resp.len() < 2 {
       return Err("SHORT RESPONSE".to_string()); 
    }

    let mut tokens = resp.split(" ");
    if tokens.next() == Some("g") {
        if let Some(url) = tokens.next() {
            if let Ok(page) = get_page(url).await {
                if let Some(body) = page.body {
                    println!("{}", body);
                    return Ok(ParseResponse::Document(body));
                }
            }
        }
    }

    println!("OH NO YOU GOOFED");

    return Err("Unable to parse response.".to_string());
}
