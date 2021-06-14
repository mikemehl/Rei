use crate::*;
use dirs::home_dir;
use std::{fs::File, io::Read};


pub fn load_marks() -> StrResult<Bookmarks> {
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
    Ok(map)
}