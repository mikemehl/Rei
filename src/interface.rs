use crate::*;


/// Functions for user interaction.
// Prompt for input and return the command.
pub fn prompt(buf: &PageBuf) -> StrResult<ParseResponse> {
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
                    "c" => ParseResponse::Clear,
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
