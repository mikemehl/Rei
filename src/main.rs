use gemini_fetch::*;
use url::*;
use tokio::*;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    page_test().await;
}

async fn page_test() {
    let _url = url::Url::parse("gemini://flounder.online/");
    match _url {
        Ok(_url) => {
            println!("WE GOOD!");
            let page = gemini_fetch::Page::fetch(&_url, None).await;
            match page {
                Ok(page) => { 
                    println!("PAGE FETCHED");
                    let body = page.body;
                    if let Some(body) = body {
                        println!("{}", body);
                    }
                }
                Err(e) => println!("ERR"),
            }
        }, 
        _ => println!("WE BAD"),
    }
    println!("EXITING.")
}
