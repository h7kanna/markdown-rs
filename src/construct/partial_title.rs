//! Title occurs in [definition][] and label end.
//!
//! They’re formed with the following BNF:
//!
//! ```bnf
//! ; Restriction: no blank lines.
//! ; Restriction: markers must match (in case of `(` with `)`).
//! title ::= marker [  *( code - '\\' | '\\' [ marker ] ) ] marker
//! marker ::= '"' | '\'' | '('
//! ```
//!
//! Titles can be double quoted (`"a"`), single quoted (`'a'`), or
//! parenthesized (`(a)`).
//!
//! Titles can contain line endings and whitespace, but they are not allowed to
//! contain blank lines.
//! They are allowed to be blank themselves.
//!
//! The title is interpreted as the [string][] content type.
//! That means that [character escapes][character_escape] and
//! [character references][character_reference] are allowed.
//!
//! ## References
//!
//! *   [`micromark-factory-title/index.js` in `micromark`](https://github.com/micromark/micromark/blob/main/packages/micromark-factory-title/dev/index.js)
//!
//! [definition]: crate::construct::definition
//! [string]: crate::content::string
//! [character_escape]: crate::construct::character_escape
//! [character_reference]: crate::construct::character_reference
//!
//! <!-- To do: link label end. -->

// To do: pass token types in.

use crate::construct::partial_whitespace::start as whitespace;
use crate::tokenizer::{Code, State, StateFnResult, TokenType, Tokenizer};

/// Type of title.
#[derive(Debug, Clone, PartialEq)]
enum Kind {
    /// In a parenthesized (`(` and `)`) title.
    Paren,
    /// In a double quoted (`"`) title.
    Double,
    /// In a single quoted (`'`) title.
    Single,
}

/// Display a marker.
fn kind_to_marker(kind: &Kind) -> char {
    match kind {
        Kind::Double => '"',
        Kind::Single => '\'',
        Kind::Paren => ')',
    }
}

/// Before a title.
///
/// ```markdown
/// |"a"
/// |'a'
/// |(a)
/// ```
pub fn start(tokenizer: &mut Tokenizer, code: Code) -> StateFnResult {
    let kind = match code {
        Code::Char('"') => Some(Kind::Double),
        Code::Char('\'') => Some(Kind::Single),
        Code::Char('(') => Some(Kind::Paren),
        _ => None,
    };

    if let Some(kind) = kind {
        tokenizer.enter(TokenType::DefinitionTitle);
        tokenizer.enter(TokenType::DefinitionTitleMarker);
        tokenizer.consume(code);
        tokenizer.exit(TokenType::DefinitionTitleMarker);
        (State::Fn(Box::new(|t, c| begin(t, c, kind))), None)
    } else {
        (State::Nok, None)
    }
}

/// After the opening marker.
///
/// This is also used when at the closing marker.
///
/// ```markdown
/// "|a"
/// '|a'
/// (|a)
/// ```
fn begin(tokenizer: &mut Tokenizer, code: Code, kind: Kind) -> StateFnResult {
    match code {
        Code::Char(char) if char == kind_to_marker(&kind) => {
            tokenizer.enter(TokenType::DefinitionTitleMarker);
            tokenizer.consume(code);
            tokenizer.exit(TokenType::DefinitionTitleMarker);
            tokenizer.exit(TokenType::DefinitionTitle);
            (State::Ok, None)
        }
        _ => {
            tokenizer.enter(TokenType::DefinitionTitleString);
            at_break(tokenizer, code, kind)
        }
    }
}

/// At something, before something else.
///
/// ```markdown
/// "|a"
/// 'a|'
/// (a|
/// b)
/// ```
fn at_break(tokenizer: &mut Tokenizer, code: Code, kind: Kind) -> StateFnResult {
    match code {
        Code::Char(char) if char == kind_to_marker(&kind) => {
            tokenizer.exit(TokenType::DefinitionTitleString);
            begin(tokenizer, code, kind)
        }
        Code::None => (State::Nok, None),
        Code::CarriageReturnLineFeed | Code::Char('\r' | '\n') => {
            tokenizer.enter(TokenType::LineEnding);
            tokenizer.consume(code);
            tokenizer.exit(TokenType::LineEnding);
            (State::Fn(Box::new(|t, c| line_start(t, c, kind))), None)
        }
        _ => {
            // To do: link.
            tokenizer.enter(TokenType::ChunkString);
            title(tokenizer, code, kind)
        }
    }
}

/// After a line ending.
///
/// ```markdown
/// "a
/// |b"
/// ```
fn line_start(tokenizer: &mut Tokenizer, code: Code, kind: Kind) -> StateFnResult {
    tokenizer.attempt(
        |t, c| whitespace(t, c, TokenType::Whitespace),
        |_ok| Box::new(|t, c| line_begin(t, c, kind)),
    )(tokenizer, code)
}

/// After a line ending, after optional whitespace.
///
/// ```markdown
/// "a
/// |b"
/// ```
fn line_begin(tokenizer: &mut Tokenizer, code: Code, kind: Kind) -> StateFnResult {
    match code {
        // Blank line not allowed.
        Code::CarriageReturnLineFeed | Code::Char('\r' | '\n') => (State::Nok, None),
        _ => at_break(tokenizer, code, kind),
    }
}

/// In title text.
///
/// ```markdown
/// "a|b"
/// ```
fn title(tokenizer: &mut Tokenizer, code: Code, kind: Kind) -> StateFnResult {
    match code {
        Code::Char(char) if char == kind_to_marker(&kind) => {
            tokenizer.exit(TokenType::ChunkString);
            at_break(tokenizer, code, kind)
        }
        Code::None | Code::CarriageReturnLineFeed | Code::Char('\r' | '\n') => {
            tokenizer.exit(TokenType::ChunkString);
            at_break(tokenizer, code, kind)
        }
        Code::Char('\\') => {
            tokenizer.consume(code);
            (State::Fn(Box::new(|t, c| escape(t, c, kind))), None)
        }
        _ => {
            tokenizer.consume(code);
            (State::Fn(Box::new(|t, c| title(t, c, kind))), None)
        }
    }
}

/// After `\`, in title text.
///
/// ```markdown
/// "a\|"b"
/// ```
fn escape(tokenizer: &mut Tokenizer, code: Code, kind: Kind) -> StateFnResult {
    match code {
        Code::Char(char) if char == kind_to_marker(&kind) => {
            tokenizer.consume(code);
            (State::Fn(Box::new(move |t, c| title(t, c, kind))), None)
        }
        _ => title(tokenizer, code, kind),
    }
}
