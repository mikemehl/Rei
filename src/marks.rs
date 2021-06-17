use crate::exec::*;
use crate::*;
use dirs::home_dir;
use std::{fs::File, io::Read};

pub fn load_marks() -> Bookmarks {
    let mut map = HashMap::new();
    if let Some(mut marks_dir) = dirs::home_dir() {
        marks_dir.push(".reimarks");
        if let Ok(mut marks_file) = File::open(marks_dir.as_os_str()) {
            let mut buf = String::new();
            marks_file.read_to_string(&mut buf);
            for line in buf.split('\n') {
                let mut split = line.split_whitespace();
                let k = split.next();
                let v = split.next();
                if k.is_some() && v.is_some() && k.unwrap().len() == 1 {
                    map.insert(k.unwrap().chars().next().unwrap(), v.unwrap().to_string());
                }
            }
        }
    }
    map
}

pub fn add_bookmark(mark: char, buf: PageBuf, marks: &mut Bookmarks) -> StrResult<()> {
    if let Some(url) = buf.url {
        marks.insert(mark, url.as_str().to_string());
        return Ok(());
    }
    Err("No page loaded to mark.")
}

pub async fn go_to_bookmark(
    mark: char,
    buf: &mut PageBuf,
    hist: &mut History,
    marks: &Bookmarks,
) -> StrResult<()> {
    if let Some((_, url)) = marks.get_key_value(&mark) {
        if let Ok(url) = url::Url::parse(url) {
            if let Ok(page) = go_url(&url).await {
                if page.body.is_some() {
                    let _ = load_page(&page, buf, hist, true);
                    buf.url = Some(url);
                    println!("{}", page.body.unwrap().len());
                    return Ok(());
                }
            }
        }
    }
    Err("Unable to load bookmark.")
}

pub fn save_bookmarks(marks: &Bookmarks) -> StrResult<()> {
    if let Some(mut marks_dir) = dirs::home_dir() {
        marks_dir.push(".reimarks");
        if let Ok(mut marks_file) = File::open(marks_dir.as_os_str()) {
            for (k, v) in marks {
                marks_file.write_fmt(format_args!("{} {}\n", k, v));
            }
            return Ok(());
        }
        return Err("Unable to write to bookmarks file.");
    }
    Err("Unable to find home directory.")
}
