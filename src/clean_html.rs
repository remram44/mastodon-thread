use lazy_static::lazy_static;
use std::collections::HashSet;
use std::iter::Peekable;
use std::str::Chars;

lazy_static! {
    static ref SAFE_TAGS: HashSet<&'static str> = [
        "P", "BR", "CODE", "BLOCKQUOTE", "PRE",
        "SUB", "SUP", "CAPTION",
        "A", "H1", "H2", "H3", "H4", "H5",
        "STRONG", "EM", "B", "U", "Q", "DEL",
        "UL", "OL", "LI", "DL", "DT", "DD",
        "TABLE", "THEAD", "TBODY", "TR", "TH", "TD",
        "COLGROUP", "COL",
    ].into_iter().collect();
}

pub fn clean_html(input: &str) -> Result<String, &'static str> {
    let mut output = String::new();
    let mut char_iter = input.chars().peekable();
    while let Some(c) = char_iter.next() {
        if c == '>' {
            output.push_str("&gt;");
        } else if c == '&' {
            entity(&mut output, &mut char_iter)?;
        } else if c == '<' {
            tag(&mut output, &mut char_iter)?;
        } else {
            output.push(c);
        }
    }
    Ok(output)
}

fn whitespace(output: &mut String, char_iter: &mut Peekable<Chars>) {
    while let Some(c) = char_iter.peek() {
        if !c.is_whitespace() {
            break;
        } else {
            output.push(char_iter.next().unwrap());
        }
    }
}

fn entity(output: &mut String, char_iter: &mut Peekable<Chars>) -> Result<(), &'static str> {
    let mut pos = 0;
    while let Some(c) = char_iter.next() {
        if c == ';' {
            output.push(c);
            return Ok(());
        } else if c == '#' && pos == 0 {
        } else if (c >= '0' && c <= '9') || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') {
        } else {
            return Err("Bad HTML entity");
        }
        output.push(c);
        pos += 1;
        if pos >= 8 {
            break;
        }
    }
    return Err("Unterminated HTML entity");
}

fn tag(output: &mut String, char_iter: &mut Peekable<Chars>) -> Result<(), &'static str> {
    whitespace(output, char_iter);
    if char_iter.peek() == Some(&'/') {
        // Closing tag
        output.push(char_iter.next().unwrap());
        whitespace(output, char_iter);
        identifier(output, char_iter)?;
        whitespace(output, char_iter);
        if char_iter.next() != Some('>') {
            return Err("Invalid closing tag");
        }
    } else {
        let mut id = identifier(output, char_iter)?;
        id.make_ascii_uppercase();
        if !SAFE_TAGS.contains(id.as_str()) {
            return Err("Unsafe tag");
        }
        whitespace(output, char_iter);
        while char_iter.peek().is_some() && char_iter.peek() != Some(&'>') {
            identifier(output, char_iter)?;
            whitespace(output, char_iter);
            if char_iter.peek() == Some(&'=') {
                char_iter.next();
                whitespace(output, char_iter);
                let Some(&c) = char_iter.peek() else {
                    return Err("Missing attribute value");
                };
                if c.is_whitespace() {
                } else if c.is_ascii_digit() {
                    number(output, char_iter);
                } else if c == '"' {
                    char_iter.next();
                    quoted_string(output, char_iter);
                } else {
                    return Err("Invalid attribute value");
                }
                whitespace(output, char_iter);
            }
        }
        if char_iter.next() != Some('>') {
            return Err("Invalid opening tag");
        }
    }
    Ok(())
}

fn identifier(output: &mut String, char_iter: &mut Peekable<Chars>) -> Result<String, &'static str> {
    let mut name = String::new();
    while let Some(&c) = char_iter.peek() {
        if !c.is_ascii_alphanumeric() {
            break;
        }
        char_iter.next();
        name.push(c);
        output.push(c);
    }
    Ok(name)
}

fn number(output: &mut String, char_iter: &mut Peekable<Chars>) {
    while let Some(&c) = char_iter.peek() {
        if !c.is_ascii_digit() {
            break;
        }
        output.push(char_iter.next().unwrap());
    }
}

fn quoted_string(output: &mut String, char_iter: &mut Peekable<Chars>)  {
    while let Some(c) = char_iter.next() {
        output.push(c);
        if c == '"' {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::clean_html;

    #[test]
    fn test_clean_html() {
        fn is_ok(input: &'static str, expected: &'static str) {
            match clean_html(input) {
                Ok(out) => assert_eq!(out, expected),
                Err(e) => panic!("Unexpected error: {}", e),
            }
        }

        fn is_err(input: &'static str, expected: &'static str) {
            match clean_html(input) {
                Ok(_) => panic!("Unexpected ok"),
                Err(e) => assert_eq!(e, expected),
            }
        }

        is_ok("<p >This is a test></p >");
        is_ok("< p>This is a test>< / p>");
        is_err("<pp>This is a test></pp>", "Unsafe tag");
        is_ok("&quot; &QUOT; &#123;");
        is_err("&toolongentitytext;", "Unterminated HTML entity");
        is_err("&<p>Test</p>", "Bad HTML entity");
        is_err("&;", "Bad HTML entity");
    }
}
