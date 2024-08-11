#![allow(dead_code)]
use chumsky::input::ValueInput;
use chumsky::prelude::*;

pub mod lexer;

pub enum Rule {
	Value,
}
pub type Ruleset = Vec<Rule>;
// pub type Span = logos::Span;
// ~=|=|!=|=~|<=|>=|<|>|\*|\+|-|\^|/|\\
enum Binary {
	Eq,
	NotEq,
	LeftMatch,
	RightMatch,
	Le,
	Ge,
	Lt,
	Gt,
	Mul,
	Add,
	Sub,
	Div,
}

enum Unary {
	Not,
	Neg,
}

enum Object {
	Node,
	Way,
	Relation,
	Area,
	Line,
	Canvas,
	Star,
}

#[derive(Debug)]
pub struct Zoom(u8, u8);

// fn expr_parser<'a,  I: Input<'a>>(
// ) -> impl Parser<'a, I, Ruleset, extra::Err<Rich<'a, Token<'a>>>> + Clone {
// 	// let zoom = select! {
// 	// 	Token::Zoom(zoom_str) => {
// 	// 		let mut zoom_str = zoom_str.split('-');
// 	// 		let a = zoom_str.next().unwrap().parse::<u8>().unwrap_or(0);
// 	// 		let b = zoom_str.next().unwrap().parse::<u8>().unwrap_or(u8::MAX);
// 	//
// 	// 		Zoom(a, b)
// 	// 	}
// 	// };
//
// 	todo()
// }

#[cfg(test)]
mod tests {
	use ariadne::{Color, Label, Report, ReportKind, Source};
	use chumsky::input::Stream;
	use chumsky::prelude::*;
	use logos::Logos;

	use crate::lexer::Token;

	use super::*;

	fn expr_parser<'a, I>() -> impl Parser<'a, I, Vec<String>, extra::Err<Rich<'a, Token<'a>>>> + Clone
	where
		I: ValueInput<'a, Token = Token<'a>, Span = SimpleSpan>,
	{
		let spaces = just(Token::Space).or(just(Token::Newline)).repeated().at_least(1).labelled("whitespace");

		// object       ::= "node" | "way" | "relation" | "area" | "line" | "canvas" | "*" | ;
		let object = select! {
			Token::Identifier("node") => "node".to_string(),
			Token::Identifier("way") => "way".to_string(),
			Token::Identifier("relation") => "relation".to_string(),
			Token::Identifier("area") => "area".to_string(),
			Token::Identifier("line") => "line".to_string(),
			Token::Identifier("canvas") => "canvas".to_string(),
			Token::Op("*") => "*".to_string(),
		}
		.labelled("object (node, way, relation, area, line, canvas, *)")
		.then_ignore(spaces.clone());

		let class = just(Token::Dot)
			// .or(just(Token::Colon))
			.then(
				select! {
					Token::Identifier(class) => class.to_string(),
				}
				.labelled("classname"),
			)
			.labelled("classname")
			.map(|(a, b)| match a {
				Token::Dot => format!(".{}", b),
				Token::Colon => format!(":{}", b),
				_ => unreachable!(),
			})
			.labelled("classname");

		let selector = class.or(object);

		let selectors = selector.repeated();
		selectors.collect()
	}

	fn print_type_of<T>(_: &T) {
		println!("{}", std::any::type_name::<T>())
	}

	// type ParserInput<'tokens, 'src> = SpannedInput<Token<'src>, Span, &'tokens [(Token<'src>, Span)]>;

	#[test]
	fn parse() {
		let file_name = "simple.mapcss";
		let src = include_str!("test/simple.mapcss");
		let token_iter = Token::lexer(src).spanned().filter_map(|(tok, span)| match tok {
			Ok(tok) => Some((tok, span.into())),
			_ => None,
		});

		let token_stream = Stream::from_iter(token_iter).spanned(SimpleSpan::from(src.len()..src.len()));

		let result = expr_parser().parse(token_stream).into_result();
		match result {
			Ok(out) => {
				println!("{:?}", out);
			}
			Err(errs) => {
				let src = Source::from(&src);
				for err in errs {
					Report::build(ReportKind::Error, file_name, err.span().start)
						// .with_code(3)
						.with_message(err.to_string())
						.with_label(
							Label::new((file_name, err.span().into_range()))
								.with_message(err.reason())
								.with_color(Color::Red),
						)
						.finish()
						.eprint((file_name, src.clone()))
						.unwrap();
				}
			}
		}
	}
}
