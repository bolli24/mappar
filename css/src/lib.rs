#![allow(dead_code)]
use crate::lexer::Token;
use chumsky::input::ValueInput;
use chumsky::prelude::*;
use ecolor::Color32;

pub mod lexer;

fn expr_parser<'a, I>() -> impl Parser<'a, I, Vec<Rule>, extra::Err<Rich<'a, Token<'a>>>> + Clone
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

	// key          ::= ID;
	let key = select! {
		Token::Identifier(key) => key.to_string(),
	};
	// tag          ::= key | tag ":" key;
	let tag = key;

	let value = select! {
		Token::Number(num) => Value::Number(num),
		Token::Identifier(ident) => Value::Identifier(ident.to_string()),
		Token::Quoted(s) => Value::String(s)
	};

	let unary = select! {
		Token::Exclamation => UnaryOp::Not,
		Token::Op("-") => UnaryOp::Neg,
	};

	let binary = select! {
		Token::Op("=") => BinaryOp::Eq,
		Token::Op("!=") => BinaryOp::NotEq,
		Token::Op("=~") => BinaryOp::RightMatch,
		Token::Op("~=") => BinaryOp::LeftMatch,
		Token::Op("<=") => BinaryOp::Le,
		Token::Op(">=") => BinaryOp::Ge,
		Token::Op("<") => BinaryOp::Lt,
		Token::Op(">") => BinaryOp::Gt,
		Token::Op("*") => BinaryOp::Mul,
		Token::Op("+") => BinaryOp::Add,
		Token::Op("-") => BinaryOp::Sub,
		Token::Op("/") => BinaryOp::Div,
	};

	let unit = select! {
		Token::Unit(unit) => Some(Unit::from(unit)),
		_ => None
	};

	let number = select! {Token::Number(n) => n};

	let hex_color = select! {Token::Color(c) => c};

	let identifier = select! {Token::Identifier(s) => s}.labelled("identifier");

	let repeated_u8 = number
		.map(|f: f64| f.round().clamp(0.0, 255.0) as u8)
		.padded_by(opt_spaces.clone())
		.separated_by(just(Token::Comma));

	let rgb_args = repeated_u8
		.clone()
		.collect_exactly::<[u8; 3]>()
		.map(|rgb| Color32::from_rgb(rgb[0], rgb[1], rgb[2]));

	let rgba_args = repeated_u8
		.collect_exactly::<[u8; 4]>()
		.map(|rgba| Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]));

	let rgb_color = just(Token::Identifier("rgb"))
		.ignore_then(rgb_args.delimited_by(just(Token::ParenOpen), just(Token::ParenClose)));

	let rgba_color = just(Token::Identifier("rgba"))
		.ignore_then(rgba_args.delimited_by(just(Token::ParenOpen), just(Token::ParenClose)));

	let color = choice((hex_color, rgb_color, rgba_color));

	let size = select! { Token::Number(number) => number}
		.then(unit)
		.map(|(number, unit)| Size { number, unit });

	let sizes = size
		.separated_by(just(Token::Comma).padded_by(opt_spaces.clone()))
		.at_least(1)
		.collect::<Vec<Size>>();

	let function_args = value
		.padded_by(opt_spaces.clone())
		.separated_by(just(Token::Comma))
		.at_least(1)
		.collect::<Vec<_>>()
		.delimited_by(just(Token::ParenOpen), just(Token::ParenClose));

	let function = identifier.then(function_args);

	// specifier    ::= named | sizes | colour | url | eval;
	let specifier = choice((
		color.map(Specifier::Color),
		function.map(|(name, args)| Specifier::Function(name.to_string(), args)),
		sizes.map(Specifier::Sizes),
		identifier.map(|s| Specifier::Named(s.to_string())),
		select! {
			Token::Quoted(s) => Specifier::Quoted(s)
		},
	))
	.labelled("specifier");

	let style = key
		.then_ignore(just(Token::Colon).padded_by(opt_spaces.clone()))
		.then(specifier)
		.map(|(key, specifier)| Style { key, specifier });

	let styleset = style
		.padded_by(opt_spaces.clone())
		.clone()
		.separated_by(just(Token::Semicolon).padded_by(opt_spaces.clone()))
		.allow_trailing()
		.collect::<Vec<Style>>();

	let declaration = styleset
		.or(empty().map(|_| Vec::new()))
		.padded_by(opt_spaces.clone())
		.delimited_by(just(Token::BraceOpen), just(Token::BraceClose))
		.padded_by(opt_spaces.clone())
		.recover_with(via_parser(nested_delimiters(
			Token::BraceOpen,
			Token::BraceClose,
			[
				(Token::ParenOpen, Token::ParenClose),
				(Token::BracketOpen, Token::BracketClose),
			],
			|_| Vec::new(),
		)))
		.labelled("declaration");

	let condition = choice((
		tag.then_ignore(opt_spaces.clone())
			.then(binary)
			.then_ignore(opt_spaces.clone())
			.then(value)
			.map(|((tag, op), value)| Condition::Binary(tag, op, value)),
		unary
			.then_ignore(opt_spaces.clone())
			.then(tag)
			.map(|(op, tag)| Condition::Unary(op, tag)),
		tag.map(Condition::Tag),
	))
	.labelled("condition");

	let test = condition.delimited_by(
		just(Token::BracketOpen).then(opt_spaces.clone()),
		opt_spaces.clone().then(just(Token::BracketClose)),
	);

	let tests = test.repeated().at_least(1).collect::<Vec<_>>();

	// object       ::= "node" | "way" | "relation" | "area" | "line" | "canvas" | "*" | ;
	let object = any()
		.filter(|t: &Token| t.try_into_object().is_some())
		.map(|t| t.try_into_object().unwrap())
		.labelled("object (node, way, relation, area, line, canvas, *, meta)");

	let class = just(Token::Dot)
		// TODO: .or(just(Token::Colon))
		.then(identifier)
		.labelled("classname")
		.map(|(a, b)| match a {
			Token::Dot => b.to_string(),
			Token::Colon => todo!(),
			_ => unreachable!(),
		});

	let maybe_class = choice((
		just(Token::Exclamation)
			.ignore_then(class.clone())
			.map(SubSelector::NotClass),
		class.map(SubSelector::Class),
	));

	let setting = just(Token::Identifier("setting"))
		.ignore_then(just(Token::Colon))
		.ignore_then(just(Token::Colon))
		.ignore_then(select! {
			Token::Identifier(s) => s
		})
		.map(|s| SubSelector::Setting(s.to_string()));

	// subselector  ::= object SPACE | object zoom | object zoom tests | maybe_class;
	let sub_selector = choice((
		object
			.then_ignore(req_space.clone())
			.map(|object| SubSelector::Object(Some(object))),
		object
			.then(tests)
			.map(|(object, tests)| SubSelector::WithTests(Some(object), tests)),
		maybe_class,
		setting,
	))
	.labelled("subselector");

	// selector     ::= subselector | selector subselector;
	let selector = sub_selector // .class1 .class2 OR .class1.class2
		.separated_by(opt_spaces.clone())
		.at_least(1)
		.collect::<Vec<_>>()
		.map(Intersection)
		.labelled("selector");

	// uSelectors   ::= selector | uSelectors "," selector;
	let union_selector = selector // .class1, .class2 OR .class1,.class2
		.separated_by(just(Token::Comma).padded_by(opt_spaces.clone()))
		.at_least(1)
		.allow_trailing()
		.collect::<Vec<_>>()
		.recover_with(via_parser(none_of(Token::BraceOpen).repeated().map(|_| Vec::new())))
		.map(Union);

	let selectors = union_selector.labelled("selectors");

	let rule = selectors
		.then(declaration)
		.map(|(selectors, styles)| Rule { selectors, styles });

	let rule_set = rule.separated_by(opt_spaces.clone()).collect();
	rule_set
}

pub type Ruleset = Vec<Rule>;

#[derive(Debug, Copy, Clone)]
pub enum BinaryOp {
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

#[derive(Debug, Copy, Clone)]
pub enum UnaryOp {
	Not,
	Neg,
}

#[derive(Debug)]
pub enum Object {
	Node,
	Way,
	Relation,
	Area,
	Line,
	Canvas,
	All,
	Meta,
}

#[derive(Debug)]
pub struct Zoom(u8, u8);

#[derive(Debug)]
pub struct Rule {
	pub selectors: Selectors,
	pub styles: Vec<Style>,
}

pub type Selectors = Union;

/// Applies to all instances with any of the subselectors
/// .class1, .class2
#[derive(Debug)]
pub struct Union(Vec<Intersection>);

/// Applies to all instances with all the subselectors
/// .class1 .class2
#[derive(Debug)]
pub struct Intersection(Vec<SubSelector>);

#[derive(Debug)]
pub enum SubSelector {
	Object(Option<Object>),
	WithTests(Option<Object>, Vec<Condition>),
	Class(String),
	NotClass(String),
	Setting(String),
}

#[derive(Debug)]
pub struct Style {
	pub key: String,
	pub specifier: Specifier,
}

#[derive(Debug)]
pub enum Specifier {
	Named(String),
	Sizes(Vec<Size>),
	Color(Color32),
	Url(String),
	Eval(String),
	Quoted(String),
	Function(String, Vec<Value>),
}

#[derive(Debug)]
pub struct Size {
	number: f64,
	unit: Option<Unit>,
}

#[derive(Debug)]
pub enum Unit {
	Px,
	Pt,
	Percent,
}

impl From<&str> for Unit {
	fn from(s: &str) -> Self {
		match s {
			"px" => Unit::Px,
			"pt" => Unit::Pt,
			"%" => Unit::Percent,
			_ => unreachable!(),
		}
	}
}

#[derive(Debug)]
pub enum Value {
	Number(f64),
	Identifier(String),
	String(String),
}

#[derive(Debug)]
pub enum Condition {
	Binary(String, BinaryOp, Value),
	Unary(UnaryOp, String),
	Tag(String),
}

#[cfg(test)]
mod tests {
	use crate::lexer::Token;
	use ariadne::{Color, Label, Report, ReportKind, Source};
	use chumsky::input::Stream;
	use chumsky::prelude::*;
	use logos::Logos;

	use super::*;

	#[test]
	fn parse() {
		let file_name = "josm.mapcss";
		let src = include_str!("test/josm.mapcss");
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
