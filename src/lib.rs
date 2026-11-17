// SPDX-License-Identifier: MIT
//! SigmaOS-genoffice-html — HTML tokenizer, sanitizer, validator, and text extractor.
//!
//! Parses HTML into a token stream, sanitizes untrusted markup for safe rendering,
//! validates tag balance, and extracts readable text. Deterministic and dependency-free.

#![forbid(unsafe_code)]
#![deny(clippy::all)]

use std::collections::BTreeMap;

/// A parsed HTML token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlToken {
    StartTag { name: String, attrs: BTreeMap<String, String>, self_closing: bool },
    EndTag { name: String },
    Text(String),
    Comment(String),
    Doctype(String),
}

const VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
    "param", "source", "track", "wbr",
];

const DROP_TAGS: &[&str] = &[
    "script", "style", "iframe", "object", "embed", "applet", "meta", "link", "base", "form",
];

fn is_void(name: &str) -> bool {
    VOID_TAGS.contains(&name)
}

fn decode_entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn parse_attrs(src: &str) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    let b = src.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        while i < b.len() && (b[i].is_ascii_whitespace() || b[i] == b'/') {
            i += 1;
        }
        let key_start = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'=' && b[i] != b'/' {
            i += 1;
        }
        if key_start == i {
            break;
        }
        let key = src[key_start..i].to_ascii_lowercase();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut value = String::new();
        if i < b.len() && b[i] == b'=' {
            i += 1;
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < b.len() && (b[i] == b'"' || b[i] == b'\'') {
                let quote = b[i];
                i += 1;
                let val_start = i;
                while i < b.len() && b[i] != quote {
                    i += 1;
                }
                value = decode_entities(&src[val_start..i]);
                if i < b.len() {
                    i += 1;
                }
            } else {
                let val_start = i;
                while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'/' {
                    i += 1;
                }
                value = decode_entities(&src[val_start..i]);
            }
        }
        attrs.insert(key, value);
    }
    attrs
}

/// Parse HTML into a flat token stream. Lenient: malformed tags become text.
pub fn parse_html(input: &str) -> Vec<HtmlToken> {
    let b = input.as_bytes();
    let mut tokens = Vec::new();
    let mut text_start = 0usize;
    let mut i = 0usize;

    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        if i > text_start {
            tokens.push(HtmlToken::Text(decode_entities(&input[text_start..i])));
        }
        if input[i..].starts_with("<!--") {
            let end = input[i + 4..].find("-->").map(|x| i + 4 + x + 3).unwrap_or(input.len());
            tokens.push(HtmlToken::Comment(input[i + 4..end.saturating_sub(3)].to_string()));
            i = end;
            text_start = i;
            continue;
        }
        if input[i..].starts_with("<!") || input[i..].starts_with("<?") {
            let end = input[i..].find('>').map(|x| i + x + 1).unwrap_or(input.len());
            let inner = input[i + 2..end.saturating_sub(1)].trim().to_string();
            tokens.push(HtmlToken::Doctype(inner));
            i = end;
            text_start = i;
            continue;
        }
        if input[i..].starts_with("</") {
            let end = input[i..].find('>').map(|x| i + x + 1).unwrap_or(input.len());
            let name = input[i + 2..end.saturating_sub(1)].trim().to_ascii_lowercase();
            if !name.is_empty() {
                tokens.push(HtmlToken::EndTag { name });
            }
            i = end;
            text_start = i;
            continue;
        }
        let mut j = i + 1;
        while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'-') {
            j += 1;
        }
        if j == i + 1 {
            // Lone '<' — treat as text.
            i += 1;
            continue;
        }
        let name = input[i + 1..j].to_ascii_lowercase();
        let tag_end_rel = input[j..].find('>').map(|x| j + x);
        let Some(tag_end) = tag_end_rel else {
            break;
        };
        let inner = input[j..tag_end].trim_end_matches('/').trim();
        let self_closing = input[j..tag_end].trim_end().ends_with('/') || is_void(&name);
        let attrs = if inner.is_empty() { BTreeMap::new() } else { parse_attrs(inner) };
        tokens.push(HtmlToken::StartTag { name, attrs, self_closing });
        i = tag_end + 1;
        text_start = i;
    }
    if text_start < input.len() {
        tokens.push(HtmlToken::Text(decode_entities(&input[text_start..])));
    }
    tokens
}

fn allowed_attr(tag: &str, attr: &str) -> bool {
    if matches!(attr, "id" | "class" | "title") {
        return true;
    }
    match tag {
        "a" => matches!(attr, "href"),
        "img" => matches!(attr, "src" | "alt" | "width" | "height"),
        "ol" => matches!(attr, "start" | "reversed" | "type"),
        "li" => matches!(attr, "value"),
        "td" | "th" => matches!(attr, "colspan" | "rowspan" | "scope"),
        "blockquote" | "q" | "del" | "ins" => matches!(attr, "cite" | "datetime"),
        "time" => matches!(attr, "datetime"),
        "data" => matches!(attr, "value"),
        _ => false,
    }
}

fn safe_url(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    !(u.starts_with("javascript:") || u.starts_with("vbscript:") || u.starts_with("data:text/html"))
}

/// Sanitize HTML for safe embedding. Returns canonical HTML fragment.
pub fn sanitize_html(input: &str) -> String {
    let tokens = parse_html(input);
    let mut out = String::new();
    let mut skip_depth = 0usize;

    for token in tokens {
        match token {
            HtmlToken::StartTag { name, attrs, self_closing } => {
                let lname = name.to_ascii_lowercase();
                if DROP_TAGS.contains(&lname.as_str()) {
                    if !self_closing && !is_void(&lname) {
                        skip_depth += 1;
                    }
                    continue;
                }
                if skip_depth > 0 {
                    if !self_closing && !is_void(&lname) {
                        skip_depth += 1;
                    }
                    continue;
                }
                out.push('<');
                out.push_str(&lname);
                for (k, v) in attrs {
                    if k.starts_with("on") || !allowed_attr(&lname, &k) {
                        continue;
                    }
                    if matches!(k.as_str(), "href" | "src") && !safe_url(&v) {
                        continue;
                    }
                    out.push_str(&format!(" {}=\"{}\"", k, escape_html(&v)));
                }
                if self_closing || is_void(&lname) {
                    out.push_str(" />");
                } else {
                    out.push('>');
                }
            }
            HtmlToken::EndTag { name } => {
                let lname = name.to_ascii_lowercase();
                if DROP_TAGS.contains(&lname.as_str()) {
                    skip_depth = skip_depth.saturating_sub(1);
                    continue;
                }
                if skip_depth > 0 {
                    continue;
                }
                if !is_void(&lname) {
                    out.push_str(&format!("</{}>", lname));
                }
            }
            HtmlToken::Text(t) => {
                if skip_depth == 0 {
                    out.push_str(&escape_html(&t));
                }
            }
            HtmlToken::Comment(_) | HtmlToken::Doctype(_) => {}
        }
    }
    out
}

const BLOCK_TAGS: &[&str] = &[
    "p", "div", "section", "article", "header", "footer", "main", "aside", "nav",
    "h1", "h2", "h3", "h4", "h5", "h6", "ul", "ol", "li", "table", "tr", "blockquote",
    "pre", "hr", "br",
];

/// Extract readable text from HTML tokens.
pub fn text_content(tokens: &[HtmlToken]) -> String {
    let mut out = String::new();
    for token in tokens {
        match token {
            HtmlToken::Text(t) => {
                out.push_str(t);
                out.push(' ');
            }
            HtmlToken::StartTag { name, .. } if name == "br" => out.push('\n'),
            HtmlToken::EndTag { name } if BLOCK_TAGS.contains(&name.as_str()) => {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            _ => {}
        }
    }
    out.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Extract plain text directly from HTML source.
pub fn html_to_text(input: &str) -> String {
    text_content(&parse_html(input))
}

/// Validate tag balance. Returns a list of problems (empty when balanced).
pub fn validate_html(input: &str) -> Vec<String> {
    let tokens = parse_html(input);
    let mut stack: Vec<String> = Vec::new();
    let mut errors = Vec::new();

    for token in tokens {
        match token {
            HtmlToken::StartTag { name, self_closing, .. } => {
                let lname = name.to_ascii_lowercase();
                if DROP_TAGS.contains(&lname.as_str()) || self_closing || is_void(&lname) {
                    continue;
                }
                stack.push(lname);
            }
            HtmlToken::EndTag { name } => {
                let lname = name.to_ascii_lowercase();
                if is_void(&lname) {
                    continue;
                }
                match stack.pop() {
                    Some(top) if top == lname => {}
                    Some(top) => {
                        errors.push(format!("unmatched closing </{lname}> (expected </{top}>)"));
                        stack.push(top);
                    }
                    None => errors.push(format!("unmatched closing </{lname}>")),
                }
            }
            _ => {}
        }
    }
    for unclosed in stack {
        errors.push(format!("unclosed <{unclosed}>"));
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let tokens = parse_html("<p class=\"x\">Hi <b>there</b></p>");
        assert!(matches!(&tokens[0], HtmlToken::StartTag { name, .. } if name == "p"));
        assert!(matches!(&tokens[1], HtmlToken::Text(t) if t == "Hi "));
        assert!(matches!(&tokens[5], HtmlToken::EndTag { name } if name == "p"));
    }

    #[test]
    fn test_sanitize_strips_script_and_handlers() {
        let dirty = "<p onclick=\"x()\">ok</p><script>alert(1)</script><a href=\"javascript:x\">bad</a>";
        let clean = sanitize_html(dirty);
        assert!(!clean.contains("script"));
        assert!(!clean.contains("onclick"));
        assert!(!clean.contains("javascript:"));
        assert!(clean.contains("<p>ok</p>"));
    }

    #[test]
    fn test_sanitize_keeps_safe_markup() {
        let clean = sanitize_html("<h1 id=\"t\">Title</h1><a href=\"https://example.invalid\">link</a><img src=\"a.png\" alt=\"A\" />");
        assert!(clean.contains("<h1 id=\"t\">Title</h1>"));
        assert!(clean.contains("href=\"https://example.invalid\""));
        assert!(clean.contains("<img alt=\"A\" src=\"a.png\" />"));
    }

    #[test]
    fn test_text_content() {
        assert_eq!(html_to_text("<h1>Title</h1><p>Hello <b>world</b></p>"), "Title\nHello world");
    }

    #[test]
    fn test_validate_balanced_and_unbalanced() {
        assert!(validate_html("<div><p>x</p></div>").is_empty());
        let errs = validate_html("<div><p>x</div>");
        assert!(!errs.is_empty());
        assert!(validate_html("<p>orphan</b>").iter().any(|e| e.contains("unmatched")));
    }

    #[test]
    fn test_entities() {
        assert_eq!(html_to_text("<p>Fish &amp; Chips &lt;3</p>"), "Fish & Chips <3");
    }
}
