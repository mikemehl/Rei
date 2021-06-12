use gemini_fetch::*;
use lazy_static::*;
use regex::Regex;
use std::{convert::TryInto, io::Write};

type StrResult<T> = Result<T, &'static str>;
/// Structures for representing the page buffer and history.
// TODO: Add more types!
enum GemTextLine {
    H1(String),
    H2(String),
    H3(String),
    Link(usize, String, url::Url),
    Line(String),
}
struct PageBuf {
    //page: Option<gemini_fetch::Page>, // The raw page response.
    lines: Vec<GemTextLine>, // The parsed lines for display.
    curr_line: usize,
    url: Option<url::Url>,
}

struct History {
    entry: Vec<url::Url>,
    curr_entry: usize,
}

impl History {
    pub fn add(self: &mut History, url: &url::Url) {
        if let Ok(url) = url::Url::parse(url.as_str()) {
            self.curr_entry += 1;
            if self.curr_entry >= self.entry.len() {
                self.entry.push(url);
                self.curr_entry = self.entry.len() - 1;
            } else {
                self.entry[self.curr_entry] = url;
                self.entry.truncate(self.curr_entry + 1);
            }
        }
    }
}

/// Enum representing all of the available commands and their associated data.
enum ParseResponse {
    GoUrl(url::Url),
    SearchBackwards(String),
    SearchForwards(String),
    FollowLink(usize), // Index of link on page.
    JumpToLine(usize),
    GoBack(usize),
    GoForward(usize),
    Print {
        use_range: bool,
        start: usize,
        stop: usize,
    },
    Enumerate {
        use_range: bool,
        start: usize,
        stop: usize,
    },
    Page(usize),    // Number of lines to page.
    History(isize), // Number of entries to show (-1 means show all)
    Invalid,
    Empty,
    Quit,
}

/// main()
#[tokio::main]
async fn main() {
    println!("Rei: A Line Mode Gemini Browser");
    let mut cont = true;
    let mut buf = PageBuf {
        lines: Vec::new(),
        curr_line: 0,
        url: None,
    };
    let mut hist = History {
        entry: Vec::new(),
        curr_entry: 0,
    };
    while cont {
        match prompt(&buf) {
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
fn prompt(buf: &PageBuf) -> StrResult<ParseResponse> {
    print!("*");
    let _ = std::io::stdout().flush();
    let mut response = String::new();
    let _bytes_read = std::io::stdin().read_line(&mut response).unwrap();
    return parse_response(response, buf);
}

// Parse the users command.
// Called by prompt to match input to commands.
fn parse_response(resp: String, buf: &PageBuf) -> StrResult<ParseResponse> {
    lazy_static! {
            static ref NUM_REGEX : regex::Regex = Regex::new(r"^([\+-]+[0-9]+|[0-9]+|\$)\s*$").unwrap();                    // Number only
            static ref NUM_LETTER_REGEX : regex::Regex = Regex::new(r"^(%|[\+-]+[0-9]+|[0-9]+)([a-z]+)\s*$").unwrap();     // Number and letter
            static ref RANGE_LETTER : regex::Regex = Regex::new(r"^([\+-]+[0-9]+|[0-9]+|\.),([\+-]+[0-9]+|[0-9]+|\$|\.)([a-z]+)\s*$").unwrap();    // Range and letter
            static ref LETTER_REGEX : regex::Regex = Regex::new(r"^([a-z\$]+)\s*$").unwrap();              // Letter only
            static ref LETTER_ARG_REGEX : regex::Regex = Regex::new(r"^([a-z])\s*([^\s]+)\s*$").unwrap(); // Letter and arg
            static ref SEARCH_REGEX : regex::Regex = Regex::new(r"^[/\?]{1}(.*)[/\?]{1}\n$").unwrap();
    }

    if resp == "\n" {
        return Ok(ParseResponse::Empty);
    }

    if NUM_REGEX.is_match(&resp) {
        if let Some(num) = NUM_REGEX.captures(&resp) {
            if let Some(num) = num.get(1) {
                return Ok(ParseResponse::JumpToLine(parse_num(
                    num.as_str(),
                    buf.lines.len(),
                    buf.curr_line,
                )));
            }
        }
    } else if NUM_LETTER_REGEX.is_match(&resp) {
        if let Some(cmds) = NUM_LETTER_REGEX.captures(&resp) {
            if let (Some(num), Some(cmd)) = (cmds.get(1), cmds.get(2)) {
                if num.as_str().starts_with("%") {
                    let cmd = cmd.as_str();
                    return Ok(match cmd {
                        "p" => ParseResponse::Print {
                            use_range: true,
                            start: 0,
                            stop: buf.lines.len(),
                        },
                        "n" => ParseResponse::Enumerate {
                            use_range: true,
                            start: 0,
                            stop: buf.lines.len(),
                        },
                        _ => ParseResponse::Invalid,
                    });
                }
                let num = parse_num(num.as_str(), buf.lines.len(), buf.curr_line);
                let cmd = cmd.as_str();
                return Ok(match cmd {
                    "p" => ParseResponse::Print {
                        use_range: true,
                        start: num,
                        stop: num,
                    },
                    "n" => ParseResponse::Enumerate {
                        use_range: true,
                        start: num,
                        stop: num,
                    },
                    _ => ParseResponse::Invalid,
                });
            }
        }
    } else if RANGE_LETTER.is_match(&resp) {
        if let Some(cmds) = RANGE_LETTER.captures(&resp) {
            if let (Some(num_start), Some(num_end), Some(cmd)) =
                (cmds.get(1), cmds.get(2), cmds.get(3))
            {
                let num_start = parse_num(num_start.as_str(), buf.lines.len(), buf.curr_line);
                let mut num_end = parse_num(num_end.as_str(), buf.lines.len(), buf.curr_line);
                if num_end < num_start {
                    num_end = num_start;
                }
                let cmd = cmd.as_str();
                return Ok(match cmd {
                    "p" => ParseResponse::Print {
                        use_range: true,
                        start: num_start,
                        stop: num_end,
                    },
                    "n" => ParseResponse::Enumerate {
                        use_range: true,
                        start: num_start,
                        stop: num_end,
                    },
                    _ => ParseResponse::Invalid,
                });
            }
        }
    } else if LETTER_REGEX.is_match(&resp) {
        if let Some(cmd) = LETTER_REGEX.captures(&resp) {
            if let Some(cmd) = cmd.get(1) {
                let cmd = cmd.as_str();
                return Ok(match cmd {
                    "p" => ParseResponse::Print {
                        use_range: true,
                        start: buf.curr_line as usize,
                        stop: buf.curr_line as usize,
                    },
                    "n" => ParseResponse::Enumerate {
                        use_range: true,
                        start: buf.curr_line as usize,
                        stop: buf.curr_line as usize,
                    },
                    "z" => ParseResponse::Page(24),
                    "q" => ParseResponse::Quit,
                    "$" => ParseResponse::JumpToLine(buf.lines.len()),
                    "b" => ParseResponse::GoBack(1),
                    "f" => ParseResponse::GoForward(1),
                    "h" => ParseResponse::History(-1),
                    _ => ParseResponse::Invalid,
                });
            }
        }
    } else if LETTER_ARG_REGEX.is_match(&resp) {
        if let Some(cmd) = LETTER_ARG_REGEX.captures(&resp) {
            if let (Some(cmd), Some(arg)) = (cmd.get(1), cmd.get(2)) {
                let cmd = cmd.as_str();
                let arg = arg.as_str();
                return match cmd {
                    "g" => parse_go_command(arg),
                    "l" => parse_link_command(arg),
                    "z" => {
                        if let Ok(size) = arg.parse::<usize>() {
                            Ok(ParseResponse::Page(size))
                        } else {
                            Ok(ParseResponse::Page(24))
                        }
                    }
                    "b" => {
                        if let Ok(depth) = arg.parse::<usize>() {
                            Ok(ParseResponse::GoBack(depth))
                        } else {
                            Ok(ParseResponse::GoBack(1))
                        }
                    }
                    "f" => {
                        if let Ok(depth) = arg.parse::<usize>() {
                            Ok(ParseResponse::GoForward(depth))
                        } else {
                            Ok(ParseResponse::GoForward(1))
                        }
                    }
                    "h" => {
                        if let Ok(depth) = arg.parse::<isize>() {
                            Ok(ParseResponse::History(depth))
                        } else {
                            Ok(ParseResponse::History(-1))
                        }
                    }
                    _ => Ok(ParseResponse::Invalid),
                };
            }
        }
    } else if SEARCH_REGEX.is_match(&resp) {
        if let Some(re) = SEARCH_REGEX.captures(&resp) {
            if let Some(re) = re.get(1) {
                if resp.starts_with("/") {
                    return Ok(ParseResponse::SearchForwards(re.as_str().to_string()));
                }
                return Ok(ParseResponse::SearchBackwards(re.as_str().to_string()));
            }
        }
    }

    return Ok(ParseResponse::Invalid);
}

fn parse_num(num: &str, mut page_length: usize, curr_line: usize) -> usize {
    if page_length < 1 {
        page_length = 1;
    }
    if num == "$" {
        page_length - 1
    } else if num == "." {
        curr_line
    } else if num.starts_with("+") {
        if let Ok(offset) = num[1..].parse::<usize>() {
            let dest = if curr_line + offset <= page_length - 1 {
                curr_line + offset
            } else {
                page_length - 1
            };
            dest
        } else {
            curr_line
        }
    } else if num.starts_with("-") {
        if let Ok(offset) = num[1..].parse::<usize>() {
            let dest = if curr_line as isize - offset as isize >= 0 {
                curr_line - offset
            } else {
                0
            };
            dest
        } else {
            curr_line
        }
    } else if let Ok(num) = num.parse::<usize>() {
        num - 1
    } else {
        0
    }
}

fn parse_go_command(url: &str) -> StrResult<ParseResponse> {
    let scheme_re = Regex::new(r"^gemini://").unwrap();
    let mut new_url = "gemini://".to_string();
    if !scheme_re.is_match(&url) {
        new_url.push_str(url);
        if let Ok(url) = url::Url::parse(&new_url) {
            return Ok(ParseResponse::GoUrl(url));
        } else {
            return Err("Unable to parse URL.");
        }
    } else if let Ok(url) = url::Url::parse(url) {
        return Ok(ParseResponse::GoUrl(url));
    } else {
        return Err("Unable to parse URL.");
    }
}

fn parse_link_command(id: &str) -> StrResult<ParseResponse> {
    if let Ok(id) = id.parse::<usize>() {
        return Ok(ParseResponse::FollowLink(id));
    }
    Err("Invalid link id.")
}

/// Command Implementations and Helpers
// Execute the users passed in command.
// Returns false if the program should terminate.
async fn execute_command(cmd: ParseResponse, buf: &mut PageBuf, hist: &mut History) -> bool {
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
                    buf.url = Some(url);
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
            if let Ok(_) = print_with_args(&cmd, buf) {
                return true;
            }
        }
        ParseResponse::Page(size) => {
            let cmd = ParseResponse::Print {
                use_range: true,
                start: buf.curr_line,
                stop: buf.curr_line + size,
            };
            if let Ok(_) = print_with_args(&cmd, buf) {
                return true;
            }
        }
        ParseResponse::FollowLink(dest_id) => {
            for line in &buf.lines {
                if let GemTextLine::Link(id, _, url) = line {
                    if *id == dest_id {
                        match go_url(&url).await {
                            Ok(page) => {
                                if page.body.is_some() {
                                    if let Ok(_) = load_page(&page, buf, hist, true) {
                                        println!("{}", page.body.unwrap().len());
                                    }
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
                hist.curr_entry = hist.curr_entry - depth;
            }
            let url: &url::Url = &hist.entry[hist.curr_entry];
            match go_url(&url).await {
                Ok(page) => {
                    if page.body.is_some() {
                        if let Ok(_) = load_page(&page, buf, hist, false) {
                            println!("{}", page.body.unwrap().len());
                        }
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
                hist.curr_entry = hist.curr_entry + 1;
            }
            let url: &url::Url = &hist.entry[hist.curr_entry];
            match go_url(&url).await {
                Ok(page) => {
                    if page.body.is_some() {
                        if let Ok(_) = load_page(&page, buf, hist, false) {
                            println!("{}", page.body.unwrap().len());
                        }
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
        ParseResponse::Quit => return false,
        ParseResponse::Empty => {
            let cmd = ParseResponse::Print {
                use_range: false,
                start: 0,
                stop: 0,
            };
            if let Ok(_) = print_with_args(&cmd, buf) {
                return true;
            }
        }
        ParseResponse::Invalid => println!("?"),
    }
    return true;
}

// Attempt to fetch a page.
async fn go_url(url: &url::Url) -> StrResult<Page> {
    if let Ok(page) = gemini_fetch::Page::fetch_and_handle_redirects(&url).await {
        return Ok(page);
    }
    return Err("Unable to fetch url.");
}

// Print part of the page (no line numbers)
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
fn load_page(
    raw: &gemini_fetch::Page,
    buf: &mut PageBuf,
    hist: &mut History,
    add_to_hist: bool,
) -> StrResult<bool> {
    if raw.header.meta.starts_with("text/gemini") {
        if let Some(body) = &raw.body {
            buf.lines.clear();
            let mut link_count: usize = 0;
            let mut lines = body.split("\n");
            buf.curr_line = 0;
            while let Some(line) = lines.next() {
                if line.starts_with("#") {
                    if let Ok(parsed) = parse_gemtext_header(line) {
                        if buf.lines.len() > 0 {
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
                }
            }
            if add_to_hist {
                hist.add(&raw.url);
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
        }
        else {
            new_url = url_str.to_string();
        }

        return new_url;
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

#[cfg(test)]
mod test {
    use crate::parse_num;

    #[test]
    fn test_num_parsing() {
        assert_eq!(parse_num("-5", 100, 50), 45);
        assert_eq!(parse_num("+5", 100, 50), 55);
        assert_eq!(parse_num("$", 100, 50), 99);
    }
}
