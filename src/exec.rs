use crate::*;
/// Command Implementations and Helpers
// Execute the users passed in command.
// Returns false if the program should terminate.
pub async fn execute_command(
    cmd: ParseResponse,
    buf: &mut PageBuf,
    hist: &mut History,
    marks: &mut Bookmarks,
) -> bool {
    match cmd {
        ParseResponse::JumpToLine(line) => {
            let page_len = buf.lines.len();
            if line < page_len {
                buf.curr_line = line;
                print_gemtext_line(&buf.lines[buf.curr_line]);
            } else {
                println!("?");
                return true;
            }
        }
        ParseResponse::GoUrl(url) => match go_url(&url).await {
            Ok(page) => {
                if page.body.is_some() {
                    let _ = load_page(&page, buf, hist, true);
                    println!("{}", page.body.unwrap().len());
                }
                return true;
            }
            Err(msg) => println!("{}", msg),
        },
        ParseResponse::Print {
            use_range: _,
            start: _,
            stop: _,
        }
        | ParseResponse::Enumerate {
            use_range: _,
            start: _,
            stop: _,
        } => {
            if print_with_args(&cmd, buf).is_ok() {
                return true;
            }
        }
        ParseResponse::Page(size) => {
            let cmd = ParseResponse::Print {
                use_range: true,
                start: buf.curr_line,
                stop: buf.curr_line + size,
            };
            if print_with_args(&cmd, buf).is_ok() {
                return true;
            }
        }
        ParseResponse::FollowLink(dest_id) => {
            for line in &buf.lines {
                if let GemTextLine::Link(id, _, url) = line {
                    if *id == dest_id {
                        match go_url(&url).await {
                            Ok(page) => {
                                if page.body.is_some() && load_page(&page, buf, hist, true).is_ok()
                                {
                                    println!("{}", page.body.unwrap().len());
                                }
                                return true;
                            }
                            Err(msg) => println!("{}", msg),
                        }
                    }
                }
            }
            return true;
        }
        ParseResponse::GoBack(mut depth) => {
            if depth < 1 {
                depth = 1;
            }
            if hist.entry.is_empty() {
                return true;
            }
            if hist.entry.len() == 1 || hist.curr_entry == 0 {
                return true;
            }
            if depth > hist.curr_entry {
                hist.curr_entry = 0;
            } else {
                hist.curr_entry -= depth;
            }
            let url: &url::Url = &hist.entry[hist.curr_entry];
            match go_url(&url).await {
                Ok(page) => {
                    if page.body.is_some() && load_page(&page, buf, hist, false).is_ok() {
                        println!("{}", page.body.unwrap().len());
                    }
                    return true;
                }
                Err(msg) => println!("{}", msg),
            }
        }
        ParseResponse::GoForward(mut depth) => {
            if depth < 1 {
                depth = 1;
            }
            if hist.entry.is_empty() {
                return true;
            }
            if hist.entry.len() == 1 || hist.curr_entry == hist.entry.len() - 1 {
                return true;
            }
            if hist.curr_entry + depth >= hist.entry.len() - 1 {
                hist.curr_entry = hist.entry.len() - 1;
            } else {
                hist.curr_entry += 1;
            }
            let url: &url::Url = &hist.entry[hist.curr_entry];
            match go_url(&url).await {
                Ok(page) => {
                    if page.body.is_some() && load_page(&page, buf, hist, false).is_ok() {
                        println!("{}", page.body.unwrap().len());
                    }
                    return true;
                }
                Err(msg) => println!("{}", msg),
            }
        }
        ParseResponse::History(depth) => {
            if depth <= 0 {
                for (i, h) in hist.entry.iter().enumerate() {
                    if i == hist.curr_entry {
                        print!(">");
                    }
                    println!("{}\t{}", i + 1, h);
                }
            } else {
                for i in 0..depth {
                    let i: usize = i.try_into().unwrap();
                    if let Some(h) = hist.entry.get(i) {
                        if i == hist.curr_entry {
                            print!(">");
                        }
                        println!("{}\t{}", i + 1, h);
                    }
                }
            }
        }
        ParseResponse::SearchForwards(re) => {
            if let Ok(re) = regex::Regex::new(re.as_str()) {
                for i in buf.curr_line..(buf.lines.len() - 1) {
                    let text = match &buf.lines[i] {
                        GemTextLine::H1(text)
                        | GemTextLine::H2(text)
                        | GemTextLine::H3(text)
                        | GemTextLine::Line(text) => text,
                        GemTextLine::Link(_, text, _) => text,
                    };
                    if re.is_match(&text) {
                        buf.curr_line = i;
                        print_gemtext_line(&buf.lines[buf.curr_line]);
                        return true;
                    }
                }
            }
            println!("?");
            return true;
        }
        ParseResponse::SearchBackwards(re) => {
            if let Ok(re) = regex::Regex::new(re.as_str()) {
                for i in 0..buf.curr_line {
                    let i = buf.curr_line - i;
                    let text = match &buf.lines[i] {
                        GemTextLine::H1(text)
                        | GemTextLine::H2(text)
                        | GemTextLine::H3(text)
                        | GemTextLine::Line(text) => text,
                        GemTextLine::Link(_, text, _) => text,
                    };
                    if re.is_match(&text) {
                        buf.curr_line = i;
                        print_gemtext_line(&buf.lines[buf.curr_line]);
                        return true;
                    }
                }
            }
            println!("?");
            return true;
        }
        ParseResponse::Clear => {
            print!("{esc}c", esc = 27 as char);
            return true;
        }
        ParseResponse::AddBookmark(name) => {
            if marks::add_bookmark(name, buf, marks).is_err() {
                println!("FAILED TO ADD BOOKMARK"); // TEMP
            }
            return true;
        }
        ParseResponse::GoBookmark(name) => {
            if marks::go_to_bookmark(name, buf, hist, marks).await.is_err() {
                println!("UNABLE TO GO TO BOOKMARK"); // TEMP
            }
            return true;
        }
        ParseResponse::Quit => return false,
        ParseResponse::Empty => {
            let cmd = ParseResponse::Print {
                use_range: false,
                start: 0,
                stop: 0,
            };
            if print_with_args(&cmd, buf).is_ok() {
                return true;
            }
        }
        ParseResponse::Invalid => println!("?"),
    }
    return true;
}

// Attempt to fetch a page.
pub async fn go_url(url: &url::Url) -> StrResult<Page> {
    if let Ok(page) = gemini_fetch::Page::fetch_and_handle_redirects(&url).await {
        return Ok(page);
    }
    return Err("Unable to fetch url.");
}

// Print part of the page
fn print_with_args(cmd: &ParseResponse, buf: &mut PageBuf) -> StrResult<bool> {
    return match cmd {
        ParseResponse::Print {
            use_range,
            start,
            stop,
        }
        | ParseResponse::Enumerate {
            use_range,
            start,
            stop,
        } => {
            if buf.lines.is_empty() {
                return Ok(true);
            }
            let start = *start;
            let stop = *stop;
            if !use_range {
                let start: usize = buf.curr_line;
                if let Some(line) = buf.lines.get(start) {
                    if let ParseResponse::Enumerate { .. } = cmd {
                        print!("{}\t", start + 1);
                    }
                    print_gemtext_line(&line);
                    buf.curr_line += 1;
                    if buf.curr_line >= buf.lines.len() {
                        buf.curr_line = buf.lines.len() - 1;
                    }
                    return Ok(true);
                }
                return Ok(false);
            } else {
                let mut start = start;
                let mut stop = stop;
                if start >= buf.lines.len() {
                    start = buf.lines.len() - 1;
                    stop = buf.lines.len();
                }

                if stop >= buf.lines.len() {
                    stop = buf.lines.len() - 1;
                }
                let print_range = start..=stop;
                for i in print_range {
                    if let ParseResponse::Enumerate { .. } = cmd {
                        print!("{}\t", i + 1);
                    }
                    if let Some(line) = buf.lines.get(i) {
                        print_gemtext_line(&line);
                    }
                }
                buf.curr_line = stop;
                return Ok(true);
            }
        }
        _ => Err("BAD THINGS HAPPENED"),
    };
}

fn print_gemtext_line(line: &GemTextLine) {
    match line {
        GemTextLine::H1(str) => println!("{}", str),
        GemTextLine::H2(str) => println!("{}", str),
        GemTextLine::H3(str) => println!("{}", str),
        GemTextLine::Line(str) => println!("{}", str),
        GemTextLine::Link(id, text, _) => println!("[{}] => {}", id, text),
    }
}

// Load a fetched page into the PageBuf and history.
pub fn load_page(
    raw: &gemini_fetch::Page,
    buf: &mut PageBuf,
    hist: &mut History,
    add_to_hist: bool,
) -> StrResult<bool> {
    if raw.header.meta.starts_with("text/gemini") {
        if let Some(body) = &raw.body {
            buf.lines.clear();
            let mut link_count: usize = 0;
            let mut lines = body.split('\n');
            buf.curr_line = 0;
            while let Some(line) = lines.next() {
                if line.starts_with('#') {
                    if let Ok(parsed) = parse_gemtext_header(line) {
                        if !buf.lines.is_empty() {
                            buf.lines.push(GemTextLine::Line("".to_string()));
                        }
                        buf.lines.push(parsed);
                        buf.lines.push(GemTextLine::Line("".to_string()));
                    }
                } else if line.starts_with("=>") {
                    if let Ok(parsed) = parse_gemtext_link(line, &mut link_count, &raw.url) {
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
                } else {
                    buf.lines.push(GemTextLine::Line(line.to_string()));
                }
            }
            if add_to_hist {
                hist.add(&raw.url);
            }
            if let Ok(new_url) = url::Url::parse(raw.url.as_str()) {
                buf.url = Some(new_url);
            }
        }
    } else {
        println!("NOT GEMINI: {}", raw.url.as_str());
        println!("{}", raw.header.meta);
        return Err("Unable to load page!");
    }
    Ok(true)
}

// Parse a gemtext header (i.e. "#{1,3}")
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

// Parse a gemtext link (i.e. "=> url [text]")
fn parse_gemtext_link(line: &str, id: &mut usize, curr_url: &url::Url) -> StrResult<GemTextLine> {
    lazy_static! {
        static ref WHITESPACE_ONLY: regex::Regex = Regex::new(r"^\s*$").unwrap();
        static ref LINK_REGEX: regex::Regex = Regex::new(r"^=>\s+([^\s]+)\s+(.+)$").unwrap();
        static ref URL_REGEX: regex::Regex = Regex::new(r"^=>\s+([^\s]+)\s*$").unwrap();
        static ref SCHEME_RE: regex::Regex = Regex::new(r"^[a-z]+://").unwrap();
    }

    fn fix_url(url_str: &str, curr_url: &url::Url) -> String {
        let mut new_url = "gemini://".to_string();
        if url_str.starts_with("gemini://") {
            new_url = url_str.to_string();
        } else if !SCHEME_RE.is_match(url_str) {
            if let Ok(joined) = curr_url.join(url_str) {
                new_url = joined.as_str().to_string();
            }
        } else {
            new_url = url_str.to_string();
        }

        new_url
    }

    if LINK_REGEX.is_match(line) {
        if let Some(captures) = LINK_REGEX.captures(line) {
            if let (Some(url_str), Some(_)) = (captures.get(1), captures.get(2)) {
                let new_url = fix_url(url_str.as_str(), curr_url);
                if let Ok(parsed_url) = url::Url::parse(new_url.as_str()) {
                    *id += 1;
                    return Ok(GemTextLine::Link(
                        *id,
                        new_url.as_str().to_string(),
                        parsed_url,
                    ));
                }
            }
        }
    } else if URL_REGEX.is_match(line) {
        if let Some(captures) = URL_REGEX.captures(line) {
            if let Some(url_str) = captures.get(1) {
                let new_url = fix_url(url_str.as_str(), curr_url);
                if let Ok(parsed_url) = url::Url::parse(new_url.as_str()) {
                    *id += 1;
                    return Ok(GemTextLine::Link(
                        *id,
                        new_url.as_str().to_string(),
                        parsed_url,
                    ));
                }
            }
        }
    }

    Err("Unable to parse link.")
}
