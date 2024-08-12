#![allow(dead_code)]
use chumsky::input::ValueInput;
use chumsky::prelude::*;
use ecolor::Color32;

pub mod lexer;

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

#[derive(Debug)]
enum Object {
	Node,
	Way,
	Relation,
	Area,
	Line,
	Canvas,
	All,
}

impl From<&str> for Object {
	fn from(s: &str) -> Self {
		match s {
			"node" => Object::Node,
			"way" => Object::Way,
			"relation" => Object::Relation,
			"area" => Object::Area,
			"line" => Object::Line,
			"canvas" => Object::Canvas,
			"*" => Object::All,
			_ => unreachable!(),
		}
	}
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

#[derive(Debug)]
pub struct Rule {
	pub selectors: Selectors,
	pub styles: Vec<Style>,
}

pub type Selectors = Vec<Union>;

#[derive(Debug)]
pub struct Union(Vec<Intersection>); // .class1, .class2

#[derive(Debug)]
pub struct Intersection(Vec<SubSelector>); // .class1 .class2

#[derive(Debug)]
pub enum SubSelector {
	Object(Object),
	Class(String),
}

#[derive(Debug)]
pub struct Style {
	pub key: String,
	pub specifier: Specifier,
}

#[derive(Debug)]
pub enum Specifier {
	Name(String),
	Sizes(Vec<Size>),
	Color(Color32),
	Url(String),
	Eval(String),
}

#[derive(Debug)]
pub struct Size {
	number: f64,
	unit: Unit,
}

#[derive(Debug)]
pub enum Unit {
	Px,
	Pt,
	Percent,
}

#[cfg(test)]
mod tests {
	use ariadne::{Color, Label, Report, ReportKind, Source};
	use chumsky::input::Stream;
	use chumsky::prelude::*;
	use logos::Logos;

	use crate::lexer::Token;

	use super::*;

	fn expr_parser<'a, I>() -> impl Parser<'a, I, Selectors, extra::Err<Rich<'a, Token<'a>>>> + Clone
	where
		I: ValueInput<'a, Token = Token<'a>, Span = SimpleSpan>,
	{
		// Required whitespace after object
		let req_space = just(Token::Space)
			.or(just(Token::Newline))
			.repeated()
			.at_least(1)
			.labelled("whitespace");
		let opt_spaces = just(Token::Space).or(just(Token::Newline)).repeated().ignored();

		// object       ::= "node" | "way" | "relation" | "area" | "line" | "canvas" | "*" | ;
		let object = any()
			.filter(|t: &Token| t.try_into_object().is_some())
			.map(|t| SubSelector::Object(t.try_into_object().unwrap()))
			.labelled("object (node, way, relation, area, line, canvas, *)")
			.then_ignore(req_space.clone());

		let class = just(Token::Dot)
			.or(just(Token::Colon))
			.then(select! {
				Token::Identifier(class) => class.to_string(),
			})
			.labelled("classname")
			.map(|(a, b)| match a {
				Token::Dot => SubSelector::Class(b),
				Token::Colon => todo!(),
				_ => unreachable!(),
			});

		let sub_selector = class.or(object).labelled("selector");
		let selector = sub_selector // .class1 .class2 OR .class1.class2
			.separated_by(opt_spaces.clone())
			.collect::<Vec<_>>()
			.map(Intersection)
			.labelled("selector");

		let union_selector = selector // .class1, .class2 OR .class1,.class2
			.separated_by(just(Token::Comma).padded_by(opt_spaces.clone()))
			.allow_trailing()
			.collect::<Vec<_>>()
			.map(Union);

		let selectors = union_selector
			.separated_by(just(Token::Comma).padded_by(opt_spaces.clone()))
			.allow_trailing();

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
