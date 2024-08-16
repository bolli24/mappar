#![allow(dead_code)]
use crate::lexer::Token;
use chumsky::input::ValueInput;
use chumsky::prelude::*;
use ecolor::Color32;

pub mod lexer;

fn expr_parser<'a, I>() -> impl Parser<'a, I, Vec<Styleset>, extra::Err<Rich<'a, Token<'a>>>> + Clone
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
	let number = select! {Token::Number(n) => n};

	let hex_color = select! {Token::Color(c) => c};

	let identifier = select! {Token::Identifier(s) => s.to_string()}.labelled("identifier");

	let quoted = select! { Token::Quoted(s) => s.to_string()};

	let key = identifier.labelled("key");
	let tag = identifier.or(quoted).labelled("tag");

	let mut value = Recursive::<_>::declare();

	let function = identifier
		.then(
			value
				.clone()
				.separated_by(just(Token::Comma).padded_by(opt_spaces.clone()))
				.collect::<Vec<_>>()
				.padded_by(opt_spaces.clone())
				.delimited_by(just(Token::ParenOpen), just(Token::ParenClose)),
		)
		.map(|(name, args)| Function(name.to_string(), args));

	value.define(function.clone().map(Value::Function).clone().or(select! {
		Token::Number(num) => Value::Number(num),
		Token::Identifier(ident) => Value::Identifier(ident.to_string()),
		Token::Quoted(s) => Value::String(s)
	}));

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

	let color = choice((hex_color, rgb_color, rgba_color)).labelled("color");
	// JOSM: "if the color is prefixed with a name and #, e.g. color: highway_track#6e541c;
	// it will appear in the Color Preference and end users can adjust the color there themself"
	let labelled_color = identifier.or_not().then(color);

	let unit = select! {
		Token::Unit(unit) => Unit::from(unit),
	};

	let size = select! { Token::Number(number) => number}
		.then(unit.or_not())
		.map(|(number, unit)| Size { number, unit });

	let sizes = size
		.separated_by(just(Token::Comma).padded_by(opt_spaces.clone()))
		.at_least(1)
		.collect::<Vec<Size>>();

	let specifier = choice((
		labelled_color.map(|(label, color)| Specifier::Color(color, label)),
		sizes.map(|sizes| {
			// This map is needed to allow parsing of single number and sizes without units
			if sizes.len() == 1 && sizes[0].unit.is_none() {
				Specifier::Value(Value::Number(sizes[0].number))
			} else {
				Specifier::Sizes(sizes)
			}
		}),
		value.clone().map(|value| match value {
			Value::Identifier(name) => Specifier::Named(name),
			_ => Specifier::Value(value),
		}),
	))
	// .map(|spec| {
	// 	println!("Parsed specifier {spec:?}");
	// 	spec
	// })
	.labelled("specifier");

	let key_specifier = key
		.then_ignore(just(Token::Colon).padded_by(opt_spaces.clone()))
		.then(specifier)
		.map(|(key, specifier)| Rule::Style(Style { key, specifier }));

	let class_assign = just(Token::Identifier("set"))
		.ignore_then(req_space.clone())
		.ignore_then(identifier)
		.map(|name| Rule::ClassAssign(name.to_string()));

	let style = choice((key_specifier, class_assign));

	let styleset = style
		.padded_by(opt_spaces.clone())
		.clone()
		.separated_by(just(Token::Semicolon).padded_by(opt_spaces.clone()))
		.allow_trailing()
		.collect::<Vec<Rule>>();

	let declaration = styleset
		.or(empty().map(|_| Vec::new())) // TODO: do we need this?
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


	// TODO: at not falsy and not truthy: !building?!  !building?
	let condition = choice((
		just(Token::Exclamation) // !function
			.ignore_then(opt_spaces.clone())
			.ignore_then(function.clone())
			.map(Condition::NotFunction),
		function.clone().map(Condition::Function), // function
		tag.then_ignore(opt_spaces.clone()) // tag binary tag
			.then(binary)
			.then_ignore(opt_spaces.clone())
			.then(value)
			.map(|((tag, op), value)| Condition::Binary(tag, op, value)),
		just(Token::Exclamation) // !tag
			.ignore_then(opt_spaces.clone())
			.ignore_then(tag)
			.map(Condition::NotTag),
		tag.then_ignore(just(Token::Question)) // tag?!
			.then_ignore(just(Token::Exclamation))
			.map(Condition::Falsy),
		tag.then_ignore(just(Token::Question)).map(Condition::Truthy), // tag?
		tag.map(Condition::Tag),                                       // tag
	))
	.map(|condition| {
		println!("Parsed condition {condition:?}");
		condition
	})
	.labelled("condition");

	let test = condition
		.padded_by(opt_spaces.clone())
		.delimited_by(just(Token::BracketOpen), just(Token::BracketClose));

	let tests = test.repeated().collect::<Vec<_>>();

	let layer = just(Token::Colon)
		.ignore_then(just(Token::Colon))
		.ignore_then(identifier)
		.map(|layer| layer.to_string())
		.or_not();

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
			.map(|object| SubSelector::Object(object, Vec::new(), None)),
		object
			.then(tests)
			.then(layer)
			.map(|((object, tests), layer)| SubSelector::Object(object, tests, layer)),
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
		.map(|(selectors, styles)| Styleset { selectors, styles });

	let rule_set = rule.separated_by(opt_spaces.clone()).collect();
	rule_set
}

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
pub struct Styleset {
	pub selectors: Selectors,
	pub styles: Vec<Rule>,
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
	Object(Object, /* Option<Zoom>,*/ Vec<Condition>, Option<Layer>),
	Class(String),
	NotClass(String),
	Setting(String),
}

type Layer = String;

#[derive(Debug)]
pub enum Rule {
	Style(Style),
	ClassAssign(String),
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
	Color(Color32, Option<String>),
	Url(String),
	Value(Value),
	Function(Function),
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
	Function(Function),
}

#[derive(Debug)]
pub enum Condition {
	Binary(String, BinaryOp, Value),
	NotTag(String),
	Truthy(String),
	Falsy(String),
	Tag(String),
	Function(Function),
	NotFunction(Function),
}

#[derive(Debug)]
// TODO: nested function calls: pub struct Function(String, Vec<Value>);
pub struct Function(String, Vec<Value>);

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
