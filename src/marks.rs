use crate::exec::*;
use crate::*;
use std::fs::OpenOptions;
use std::io::Read;

pub fn load_marks() -> Bookmarks {
    let mut map = HashMap::new();
    if let Some(mut marks_dir) = dirs::home_dir() {
        marks_dir.push(".reimarks");
        if let Ok(mut marks_file) = OpenOptions::new()
            .read(true)
            .write(true)
            .open(marks_dir.as_os_str())
        {
            let mut buf = String::new();
            if marks_file.read_to_string(&mut buf).is_err() {
                return map;
            }
            for line in buf.split('\n') {
                let mut split = line.split_whitespace();
                let k = split.next();
                let v = split.next();
                if let (Some(k), Some(v)) = (k,v) {
                    if let Some(k) = k.chars().nth(0) {
                        map.insert(k, v.to_string());
                    }
                }
            }
        }
    }
    map
}

pub fn add_bookmark(mark: char, buf: &mut PageBuf, marks: &mut Bookmarks) -> StrResult<()> {
    if let Some(url) = &buf.url {
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
        if let Ok(mut marks_file) = OpenOptions::new().write(true).open(marks_dir.as_os_str()) {
            for (k, v) in marks {
                if marks_file.write_fmt(format_args!("{} {}\n", k, v)).is_ok() {
                    println!("Saving bookmark {}: {}", k, v);
                }
            }
            return Ok(());
        }
        return Err("Unable to write to bookmarks file.");
    }
    Err("Unable to find home directory.")
}
