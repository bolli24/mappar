use crate::{Object, Regex};
use ecolor::{Color32, ParseHexColorError};
use logos::{FilterResult, Lexer, Logos, Skip};
use std::fmt::Display;
use std::num::ParseFloatError;
use thiserror::Error;

#[derive(Logos, Debug, Clone, PartialEq, Eq)]
#[logos(skip r"[\f]+")]
#[logos(error = LexingError)]
#[logos(extras = (usize, bool))]
pub enum Token<'a> {
	#[token("{")]
	BraceOpen,
	#[token("}")]
	BraceClose,
	#[token("[")]
	BracketOpen,
	#[token("]")]
	BracketClose,
	#[token("(")]
	ParenOpen,
	#[token(")")]
	ParenClose,

	#[token(".")]
	Dot,
	#[token(":")]
	Colon,
	#[token(",")]
	Comma,
	#[token(";")]
	Semicolon,
	#[token("|")]
	Pipe,
	#[token("@")]
	At,
	#[token("!")]
	Exclamation,
	#[token("?")]
	Question,
	#[token("$")]
	Dollar,
	#[token("⧉")]
	Overlap,
	#[token("∈")]
	ElementOf,
	#[token("⊆")]
	SubsetOrEqual,
	#[token("⊇")]
	SupersetOrEqual,
	#[token("⊈")]
	NotSubsetOrEqual,
	#[token("⊉")]
	NotSupersetOrEqual,

	#[regex(r"\t| ")]
	Space,

	#[regex(r"\|z\d{1,2}-\d{1,2}|\|z\d{1,2}-|\|z-\d{1,2}|\|z\d{1,2}", |lex| &lex.slice()[2..], priority = 3)]
	Zoom(&'a str),

	// #[token("/", |lex| lex.slice(), priority = 2)]
	#[regex(r"~=|=|!=|=~|!~|<=|>=|<|>|\^=|\*|\+|-|\^|/|\\", |lex| lex.slice(), priority = 3)]
	Op(&'a str),

	#[regex(
		r"#(?:[0-9a-fA-F]{2}_){3}[0-9a-fA-F]{2}|#(?:[0-9a-fA-F]{3}){1,2}(?:[0-9a-fA-F]{1,2})?",
		color
	)]
	Color(Color32),

	// TODO: maybe add support for rgba, rgb directly here
	#[regex(r"px|pt|%", |lex| lex.slice(), priority = 2)]
	Unit(&'a str),

	#[regex(r"rad|deg|°|grad|turn", |lex| lex.slice(), priority = 2)]
	RotationUnit(&'a str),

	#[regex(r"[[^[:ascii:]][[:alpha:]]_][[^[:ascii:]][[:alnum:]]_-]*", |lex| lex.slice(), priority = 1)]
	Identifier(&'a str),

	#[regex(r"[-+]?(?:(?:0|[1-9]\d*)(?:\.\d+)?|\.\d+)(?:[eE][+-]?\d+)?", |lex| lex.slice(), priority = 2)]
	// Number(F64),
	Number(&'a str),

	#[regex(r#"[\"']"#, string)]
	Quoted(String),

	#[regex(r"/(?:\\.|[^/\\\n*])(?:\\.|[^/\\\n])*/i?", regex, priority = 4)]
	Regex(Regex),
	#[regex(r"/\*", multiline_comment)]
	Comment,

	#[regex(r"\r\n|\r|\n", |lex| lex.extras.0 += 1)]
	Newline,
}

impl Token<'_> {
	#[must_use]
	pub const fn is_whitespace(&self) -> bool {
		matches!(self, Token::Space | Token::Newline)
	}
	#[must_use]
	pub fn try_into_object(&self) -> Option<Object> {
		match self {
			Token::Identifier("node") => Some(Object::Node),
			Token::Identifier("way") => Some(Object::Way),
			Token::Identifier("relation") => Some(Object::Relation),
			Token::Identifier("area") => Some(Object::Area),
			Token::Identifier("canvas") => Some(Object::Canvas),
			Token::Op("*") => Some(Object::All),
			Token::Identifier("meta") => Some(Object::Meta),
			_ => None,
		}
	}
}

// #[derive(Debug, Clone)]
// pub struct F64(f64);
//
// impl Hash for F64 {
// 	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
// 		self.0.to_bits().hash(state)
// 	}
// }
//
// impl PartialEq<Self> for F64 {
// 	fn eq(&self, other: &Self) -> bool {
// 		self.0.to_bits() == other.0.to_bits()
// 	}
// }
//
// impl Eq for F64 {}
//
// impl From<f64> for F64 {
// 	fn from(num: f64) -> Self {
// 		F64(num)
// 	}
// }
//
// impl From<F64> for f64 {
// 	fn from(num: F64) -> f64 {
// 		num.0
// 	}
// }

impl Display for Token<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Token::BraceOpen => write!(f, "{{"),
			Token::BraceClose => write!(f, "}}"),
			Token::BracketOpen => write!(f, "["),
			Token::BracketClose => write!(f, "]"),
			Token::ParenOpen => write!(f, "("),
			Token::ParenClose => write!(f, ")"),
			Token::Dot => write!(f, "."),
			Token::Colon => write!(f, ":"),
			Token::Comma => write!(f, ","),
			Token::Semicolon => write!(f, ";"),
			Token::Pipe => write!(f, "|"),
			Token::At => write!(f, "@"),
			Token::Exclamation => write!(f, "!"),
			Token::Question => write!(f, "?"),
			Token::Dollar => write!(f, "$"),
			Token::Overlap => write!(f, "⧉"),
			Token::ElementOf => write!(f, "∈"),
			Token::SubsetOrEqual => write!(f, "⊆"),
			Token::SupersetOrEqual => write!(f, "⊇"),
			Token::NotSubsetOrEqual => write!(f, "⊈"),
			Token::NotSupersetOrEqual => write!(f, "⊉"),
			Token::Space => write!(f, " "),
			Token::Zoom(zoom) => write!(f, "|z{zoom}"),
			Token::Op(op) => write!(f, "{op}"),
			Token::Color(color) => write!(f, "{color:?}"),
			Token::Unit(unit) => write!(f, "{unit}"),
			Token::RotationUnit(unit) => write!(f, "{unit}"),
			Token::Identifier(ident) => write!(f, "{ident}"),
			Token::Number(num) => write!(f, "{num}"),
			Token::Quoted(string) => write!(f, "\"{string}\""),
			Token::Regex(regex) => write!(f, "/{}/{}", regex.0, if regex.1 { "i" } else { "" }),
			Token::Comment => write!(f, "/*"),
			Token::Newline => writeln!(f),
		}
	}
}

impl From<ParseFloatError> for LexingError {
	fn from(err: ParseFloatError) -> Self {
		Self::InvalidNumber(format!("{err}"))
	}
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Error)]
pub enum LexingError {
	#[default]
	#[error("Unexpected character")]
	UnexpectedCharacter,
	#[error("Invalid number: {0}")]
	InvalidNumber(String),
	#[error("Incomplete comment")]
	IncompleteComment,
	#[error("Invalid hex escape")]
	HexEscapeError,
	#[error("Invalid hex color: {0:?}")]
	ParseHexColorError(ParseHexColorError),
}

fn multiline_comment<'a>(lex: &mut Lexer<'a, Token<'a>>) -> FilterResult<(), LexingError> {
	enum State {
		ExpectStar,
		ExpectSlash,
	}
	let remainder = lex.remainder();
	let (mut state, mut iter) = (State::ExpectStar, remainder.chars());
	while let Some(next_char) = iter.next() {
		match next_char {
			'\n' => {
				lex.extras.0 += 1;
				state = State::ExpectStar;
			}
			'*' => state = State::ExpectSlash,
			'/' if matches!(state, State::ExpectSlash) => {
				lex.bump(remainder.len() - iter.as_str().len());
				return FilterResult::Skip;
			}
			_ => state = State::ExpectStar,
		}
	}
	lex.bump(remainder.len());
	FilterResult::Error(LexingError::IncompleteComment)
}

const fn newline<'a>(lex: &mut Lexer<'a, Token<'a>>) -> Skip {
	lex.extras.0 += 1;
	Skip
}

fn string<'a>(lex: &mut Lexer<'a, Token<'a>>) -> String {
	let quote = lex.slice().chars().next().unwrap();
	let mut iter = lex.remainder().chars();
	let mut string = String::new();
	while let Some(next_char) = iter.next() {
		if next_char == quote {
			lex.bump(lex.remainder().len() - iter.as_str().len());
			return string;
		}
		string.push(next_char);
	}
	lex.bump(lex.remainder().len());
	string
}

fn regex<'a>(lex: &Lexer<'a, Token<'a>>) -> Regex {
	let slice = lex.slice();
	let case_insensitive = slice.ends_with('i');
	let trim_end = if case_insensitive { 2 } else { 1 };
	let pattern = &slice[1..slice.len() - trim_end];

	Regex(pattern.to_string(), case_insensitive)
}

fn color<'a>(lex: &Lexer<'a, Token<'a>>) -> Result<Color32, LexingError> {
	let slice = lex.slice();
	if slice.as_bytes().contains(&b'_') {
		let mut cleaned = String::with_capacity(slice.len());
		for ch in slice.chars() {
			if ch != '_' {
				cleaned.push(ch);
			}
		}
		return Color32::from_hex(&cleaned).map_err(LexingError::ParseHexColorError);
	}
	Color32::from_hex(slice).map_err(LexingError::ParseHexColorError)
}

#[cfg(test)]
mod tests {
	use super::Token;
	use crate::Regex;
	use ecolor::Color32;
	use logos::Logos;
	use std::fs::File;
	use std::io::Write;

	#[test]
	fn test_binary_op() {
		let mut lex = Token::lexer("~==!==~!~<=>=<>*/+-^=^\\");

		assert_eq!(lex.next(), Some(Ok(Token::Op("~="))));
		assert_eq!(lex.next(), Some(Ok(Token::Op("="))));
		assert_eq!(lex.next(), Some(Ok(Token::Op("!="))));
		assert_eq!(lex.next(), Some(Ok(Token::Op("=~"))));
		assert_eq!(lex.next(), Some(Ok(Token::Op("!~"))));
		assert_eq!(lex.next(), Some(Ok(Token::Op("<="))));
		assert_eq!(lex.next(), Some(Ok(Token::Op(">="))));
		assert_eq!(lex.next(), Some(Ok(Token::Op("<"))));
		assert_eq!(lex.next(), Some(Ok(Token::Op(">"))));
		assert_eq!(lex.next(), Some(Ok(Token::Op("*"))));
		assert_eq!(lex.next(), Some(Ok(Token::Op("/"))));
		assert_eq!(lex.next(), Some(Ok(Token::Op("+"))));
		assert_eq!(lex.next(), Some(Ok(Token::Op("-"))));
		assert_eq!(lex.next(), Some(Ok(Token::Op("^="))));
		assert_eq!(lex.next(), Some(Ok(Token::Op("^"))));
		assert_eq!(lex.next(), Some(Ok(Token::Op("\\"))));
		assert_eq!(lex.next(), None);
	}

	#[test]
	fn test_unary_op() {
		let mut lex = Token::lexer("-!");
		assert_eq!(lex.next(), Some(Ok(Token::Op("-"))));
		assert_eq!(lex.next(), Some(Ok(Token::Exclamation)));
		assert_eq!(lex.next(), None);
	}

	#[test]
	fn test_braces() {
		let mut lex = Token::lexer("{}[]()");
		assert_eq!(lex.next(), Some(Ok(Token::BraceOpen)));
		assert_eq!(lex.next(), Some(Ok(Token::BraceClose)));
		assert_eq!(lex.next(), Some(Ok(Token::BracketOpen)));
		assert_eq!(lex.next(), Some(Ok(Token::BracketClose)));
		assert_eq!(lex.next(), Some(Ok(Token::ParenOpen)));
		assert_eq!(lex.next(), Some(Ok(Token::ParenClose)));
		assert_eq!(lex.next(), None);
	}

	#[test]
	fn hex_color() {
		let mut lex = Token::lexer("#fff#000#f00#ff0000#68_0B_AB_FF");
		assert_eq!(lex.next(), Some(Ok(Token::Color(Color32::from_hex("#fff").unwrap()))));
		assert_eq!(lex.next(), Some(Ok(Token::Color(Color32::from_hex("#000").unwrap()))));
		assert_eq!(lex.next(), Some(Ok(Token::Color(Color32::from_hex("#f00").unwrap()))));
		assert_eq!(
			lex.next(),
			Some(Ok(Token::Color(Color32::from_hex("#ff0000").unwrap())))
		);
		assert_eq!(
			lex.next(),
			Some(Ok(Token::Color(Color32::from_hex("#680BABFF").unwrap())))
		);
		assert_eq!(lex.next(), None);
	}

	#[test]
	fn numbers() {
		let mut lex = Token::lexer("4.01\n-456.8\n0.0\n+0.0\n-0.0\n.60\n10e3\n-3.4e-2\n12\n-1");
		let numbers = [
			"4.01", "-456.8", "0.0", "+0.0", "-0.0", ".60", "10e3", "-3.4e-2", "12", "-1",
		];
		for number in numbers {
			assert_eq!(lex.next(), Some(Ok(Token::Number(number))));
			lex.next();
		}
		assert_eq!(lex.next(), None);
	}

	#[test]
	fn identifiers() {
		let mut lex = Token::lexer("🔥dja🔥ఏdada1d");
		assert_eq!(lex.next(), Some(Ok(Token::Identifier("🔥dja🔥ఏdada1d"))));
		assert_eq!(lex.next(), None);
	}

	#[test]
	fn regex_token() {
		let mut lex = Token::lexer("/foo\\/bar\\d+/");
		assert_eq!(
			lex.next(),
			Some(Ok(Token::Regex(Regex("foo\\/bar\\d+".to_string(), false))))
		);
		assert_eq!(lex.next(), None);
	}

	fn lex_file(name: &str, source: &str) -> (usize, usize) {
		let mut lex = Token::lexer(source);
		let file = File::create(format!("test_output_{name}.txt")).unwrap();
		let mut writer = std::io::BufWriter::new(file);
		let mut tokens = 0;
		let mut errors = 0;
		while let Some(token) = lex.next() {
			match token {
				Ok(token) => {
					tokens += 1;
					writer.write_all(format!("{:?}\n", token).as_bytes()).unwrap();
				}
				Err(error) => {
					errors += 1;
					writer
						.write_all(format!("Error at line {}: {:?}\n", lex.extras.0 + 1, error).as_bytes())
						.unwrap();
				}
			}
		}
		(tokens, errors)
	}

	#[test]
	fn mapcss_files() {
		let sources = [
			("josm", include_str!("test/josm.mapcss")),
			("apple", include_str!("test/apple.mapcss")),
		];
		for (name, source) in sources {
			let (tokens, errors) = lex_file(name, source);
			println!("{name}: {tokens} tokens, {errors} errors");
		}
	}
}
