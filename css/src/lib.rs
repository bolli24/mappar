#![allow(dead_code)]
use crate::lexer::Token;
use chumsky::input::ValueInput;
use chumsky::prelude::*;
use ecolor::Color32;
use std::{
	fmt::{Debug, Display},
	u8,
};

pub mod lexer;

// TODO: other operators, proper regex, parent relation

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
	let number = select! {Token::Number(n) => n}
		.try_map(|n, span| n.parse::<f64>().map_err(|_| Rich::custom(span, "Invalid number.")));

	let hex_color = select! {Token::Color(c) => c};

	let identifier = select! {Token::Identifier(s) => s.to_string()}.labelled("identifier");

	let quoted = select! { Token::Quoted(s) => s.to_string()};

	let colon_tag = identifier
		.separated_by(just(Token::Colon))
		.at_least(1)
		.collect::<Vec<_>>()
		.map(|v| v.join(":"));

	let key = identifier.labelled("key");
	let tag = colon_tag.or(quoted).labelled("tag");

	let binary = select! {
		Token::Op("=") => BinaryOp::Eq,
		Token::Op("!=") => BinaryOp::NotEq,
		Token::Op("=~") => BinaryOp::RegexMatch,
		Token::Op("~=") => BinaryOp::ListMembership,
		Token::Op("<=") => BinaryOp::Le,
		Token::Op(">=") => BinaryOp::Ge,
		Token::Op("<") => BinaryOp::Lt,
		Token::Op(">") => BinaryOp::Gt,
		Token::Op("*") => BinaryOp::Mul,
		Token::Op("+") => BinaryOp::Add,
		Token::Op("-") => BinaryOp::Sub,
		Token::Op("/") => BinaryOp::Div,
	};

	let unary = select! {
		Token::Op("-") => UnaryOp::Neg,
		Token::Op("!") => UnaryOp::Not
	};

	let mut expression = Recursive::<_>::declare();

	let function = identifier
		.then(
			expression
				.clone()
				.separated_by(just(Token::Comma).padded_by(opt_spaces.clone()))
				.collect::<Vec<_>>()
				.padded_by(opt_spaces.clone())
				.delimited_by(just(Token::ParenOpen), just(Token::ParenClose)),
		)
		.map(|(name, args)| Function(name.to_string(), args));

	/*
	let unary = op('-')
	.repeated()
	.then(atom)
	.foldr(|_op, rhs| Expr::Neg(Box::new(rhs)));

	let product = unary.clone()
		.then(op('*').to(Expr::Mul as fn(_, _) -> _)
			.or(op('/').to(Expr::Div as fn(_, _) -> _))
			.then(unary)
			.repeated())
		.foldl(|lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)));

	let sum = product.clone()
		.then(op('+').to(Expr::Add as fn(_, _) -> _)
			.or(op('-').to(Expr::Sub as fn(_, _) -> _))
			.then(product)
			.repeated())
		.foldl(|lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)));
	 */

	// TODO: add error for not supported binary operators

	let atom = choice((
		function.clone().map(Expression::Function),
		select! {
			Token::Identifier(ident) => Expression::Identifier(ident.to_string()),
			Token::Quoted(s) => Expression::String(s)
		},
		number.map(Expression::Number),
		expression.clone().delimited_by(
			just(Token::ParenOpen).padded_by(opt_spaces.clone()),
			just(Token::ParenClose).padded_by(opt_spaces.clone()),
		),
	));

	let unary_expression = unary
		.repeated()
		.foldr(atom.clone(), |op, expr| Expression::Unary(Box::new(expr), op));

	let factor = unary_expression.clone().then_ignore(opt_spaces.clone()).foldl(
		binary
			.filter(|op| matches!(op, BinaryOp::Mul | BinaryOp::Div))
			.then_ignore(opt_spaces.clone())
			.then(unary_expression.clone())
			.repeated(),
		|a, (op, b)| Expression::Binary(Box::new(a), op, Box::new(b)),
	);

	let term = factor.clone().then_ignore(opt_spaces.clone()).foldl(
		binary
			.filter(|op| matches!(op, BinaryOp::Add | BinaryOp::Sub))
			.then_ignore(opt_spaces.clone())
			.then(factor.clone())
			.repeated(),
		|a, (op, b)| Expression::Binary(Box::new(a), op, Box::new(b)),
	);

	/*
	   let atom = int
	   .or(expr.delimited_by(just('('), just(')')))
	   .or(call)
	   .or(ident.map(Expr::Var));
	*/

	expression.define(term.labelled("expression"));

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

	let size = number.then(unit.or_not()).map(|(number, unit)| Size { number, unit });

	let sizes = size
		.separated_by(just(Token::Comma).padded_by(opt_spaces.clone()))
		.at_least(1)
		.collect::<Vec<Size>>()
		.filter(|sizes| sizes.len() > 1 || sizes[0].unit.is_some()); // A single size without a unit is a number

	let specifier = choice((
		labelled_color.map(|(label, color)| Specifier::Color(color, label)),
		sizes.map(Specifier::Sizes),
		expression.clone().map(|expression| match expression {
			Expression::Identifier(name) => Specifier::Named(name),
			_ => Specifier::Expression(expression),
		}),
	))
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

	let escaped_slash = just(Token::Op("\\"))
		.then_ignore(just(Token::Op("\\")))
		.map(|_| "\\\\")
		.or(just(Token::Op("\\")).then_ignore(just(Token::Op("/"))).map(|_| "\\/"));

	let regex_chars = escaped_slash.or(select!(
		Token::Identifier(name) => name,
		Token::Op(op) if op != "/" => op,
		Token::Number(number) => number,
		Token::Unit(unit) => unit,
		Token::Space => " ",
		Token::Dot => ".",
		Token::Colon => ":",
		Token::Comma => ",",
		Token::Semicolon => ";",
		Token::Pipe => "|",
		Token::At => "@",
		Token::Exclamation => "!",
		Token::Question => "?",
		Token::Dollar => "$",
		Token::ParenOpen => "(",
		Token::ParenClose => ")",
		Token::BracketOpen => "[",
		Token::BracketClose => "]",
		Token::BraceOpen => "{",
		Token::BraceClose => "}",
	));

	let regex = just(Token::Op("/"))
		.ignore_then(
			regex_chars
				.repeated()
				.collect::<Vec<&str>>()
				.map(|strings| strings.join("")),
		)
		.then_ignore(just(Token::Op("/")));

	let binary_condition = tag
		.clone()
		.then_ignore(opt_spaces.clone())
		.then(binary)
		.then_ignore(opt_spaces.clone())
		.then(expression.clone());

	let regex_condition = tag
		.clone()
		.then_ignore(opt_spaces.clone())
		.then_ignore(just(Token::Op("=~")))
		.then_ignore(opt_spaces.clone())
		.then(regex.clone());

	// TODO: at not falsy and not truthy: !building?!  !building?
	let condition = recursive(|condition| {
		choice((
			regex_condition.map(|(name, regex)| Condition::Regex(name, regex)),
			function.clone().map(Condition::Function), // function
			binary_condition
				.clone() // tag binary tag
				.map(|((tag, op), value)| Condition::Binary(tag, op, value)),
			tag.clone()
				.then_ignore(just(Token::Question)) // tag?!
				.then_ignore(just(Token::Exclamation))
				.map(Condition::Falsy),
			tag.clone().then_ignore(just(Token::Question)).map(Condition::Truthy), // tag?
			tag.clone().map(Condition::Tag),                                       // tag
			just(Token::Exclamation) // !condition
				.ignore_then(opt_spaces.clone())
				.ignore_then(condition)
				.try_map(|c, span| match c {
					Condition::Binary(_, _, _) => Err(Rich::custom(span, "Can't negate binary expressions.")),
					_ => Ok(Condition::Not(Box::new(c))),
				}),
		))
	})
	.labelled("condition");

	let test = condition
		.padded_by(opt_spaces.clone())
		.delimited_by(just(Token::BracketOpen), just(Token::BracketClose));

	let tests = test.repeated().collect::<Vec<_>>();

	let maybe_layer = just(Token::Colon)
		.ignore_then(just(Token::Colon))
		.ignore_then(identifier)
		.or_not();

	// object       ::= "node" | "way" | "relation" | "area" | "line" | "canvas" | "*" | ;
	let object = any()
		.filter(|t: &Token| t.try_into_object().is_some())
		.map(|t| t.try_into_object().unwrap())
		.labelled("object (node, way, relation, area, line, canvas, *, meta)");

	let class_name = just(Token::Dot).ignore_then(identifier).labelled("classname");
	let pseudo_class = just(Token::Colon).ignore_then(identifier).labelled("pseudoclassname");

	let maybe_class = choice((
		just(Token::Exclamation)
			.ignore_then(class_name.clone())
			.map(SubSelector::NotClass),
		class_name.map(SubSelector::Class),
		just(Token::Exclamation)
			.ignore_then(pseudo_class.clone())
			.map(SubSelector::NotClass),
		pseudo_class.clone().map(SubSelector::PseudoClass),
	));

	let maybe_pseudo_class = pseudo_class.clone().or_not();

	let zoom = select!(Token::Zoom(string) => Zoom::try_from(string))
		.try_map(|zoom, span| zoom.map_err(|_| Rich::custom(span, "Invalid zoom specifier: {s}")))
		.or_not();

	let setting = just(Token::Identifier("setting"))
		.ignore_then(just(Token::Colon))
		.ignore_then(just(Token::Colon))
		.ignore_then(select! {
			Token::Identifier(s) => s
		})
		.map(|s| SubSelector::Setting(s.to_string()));

	let sub_selector = choice((
		group((object, zoom, pseudo_class, tests.clone())) // object zoom? pseudoClass tests?
			.map(|(object, zoom, p_class, tests)| SubSelector::Object(object, zoom, tests, Some(p_class))),
		group((object, zoom, tests, maybe_pseudo_class)) // object zoom? tests? pseudoClass?
			.map(|(object, zoom, tests, p_class)| SubSelector::Object(object, zoom, tests, p_class)),
		object
			.then_ignore(req_space.clone())
			.map(|object| SubSelector::Object(object, None, Vec::new(), None)),
		maybe_class,
		setting,
	))
	.labelled("subselector");

	let property = identifier.try_map(|name, span| match name.as_str() {
		"index" => Ok(Property::Index),
		"role" => Ok(Property::Role),
		_ => Err(Rich::custom(
			span,
			format!("Invalid property {name} in link condition. Must be either 'index' or 'role'."),
		)),
	});

	let link = property
		.then_ignore(opt_spaces.clone())
		.then(binary)
		.then_ignore(opt_spaces.clone())
		.then(expression)
		.padded_by(opt_spaces.clone())
		.delimited_by(just(Token::BracketOpen), just(Token::BracketClose))
		.try_map(|((a, op), b), span| {
			if LINK_OPERATIONS.contains(&op) {
				Ok(Link(a, op, b))
			} else {
				Err(Rich::custom(span, format!("Invalid operator {op}")))
			}
		})
		.or_not();

	let child_relation = sub_selector
		.clone()
		.then_ignore(opt_spaces.clone())
		.then_ignore(just(Token::Op(">")))
		.then(link)
		.then_ignore(opt_spaces.clone())
		.then(sub_selector.clone())
		.map(|((a, link), b)| ChildRelation(Box::new(a), Box::new(b), link));

	//	selector     ::= subselector | child;
	let selector = choice((
		child_relation.map(Selector::ChildRelation),
		sub_selector.clone().map(Selector::SubSelector),
	)) // .class1 .class2 OR .class1.class2
	.separated_by(opt_spaces.clone())
	.at_least(1)
	.collect::<Vec<_>>()
	.then(maybe_layer)
	.map(|(selectors, maybe_layer)| Intersection(selectors, maybe_layer))
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

	let rule_set = rule
		.separated_by(opt_spaces.clone())
		.collect()
		.padded_by(opt_spaces.clone());
	rule_set
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BinaryOp {
	Eq,
	NotEq,
	ListMembership,
	RegexMatch,
	Le,
	Ge,
	Lt,
	Gt,
	Add,
	Sub,
	Mul,
	Div,
}

/// Operations permitted in expressions
const EXPRESSION_BINARY_OPS: [BinaryOp; 4] = [BinaryOp::Mul, BinaryOp::Add, BinaryOp::Mul, BinaryOp::Div];
const EXPRESSION_UNARY_OPS: [UnaryOp; 1] = [UnaryOp::Neg];

impl Display for BinaryOp {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			BinaryOp::Eq => write!(f, "="),
			BinaryOp::NotEq => write!(f, "!="),
			BinaryOp::ListMembership => write!(f, "~="),
			BinaryOp::RegexMatch => write!(f, "=~"),
			BinaryOp::Le => write!(f, "<="),
			BinaryOp::Ge => write!(f, ">="),
			BinaryOp::Lt => write!(f, "<"),
			BinaryOp::Gt => write!(f, ">"),
			BinaryOp::Mul => write!(f, "*"),
			BinaryOp::Add => write!(f, "+"),
			BinaryOp::Sub => write!(f, "-"),
			BinaryOp::Div => write!(f, "/"),
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub enum UnaryOp {
	Not,
	Neg,
}

impl Display for UnaryOp {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			UnaryOp::Not => write!(f, "!"),
			UnaryOp::Neg => write!(f, "-"),
		}
	}
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

impl TryFrom<&str> for Zoom {
	type Error = ();

	fn try_from(value: &str) -> Result<Self, Self::Error> {
		match value.split("-").collect::<Vec<_>>().as_slice() {
			[z] => {
				let z = z.parse::<u8>().map_err(|_| ())?;
				Ok(Zoom(z, z))
			}
			[z1, ""] => {
				let z1 = z1.parse::<u8>().map_err(|_| ())?;
				Ok(Zoom(z1, u8::MAX))
			}
			["", z2] => {
				let z2 = z2.parse::<u8>().map_err(|_| ())?;
				Ok(Zoom(0, z2))
			}
			[z1, z2] => {
				let z1 = z1.parse::<u8>().map_err(|_| ())?;
				let z2 = z2.parse::<u8>().map_err(|_| ())?;
				Ok(Zoom(z1, z2))
			}
			_ => Err(()),
		}
	}
}

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
/// .class1.class2 or .class1 .class2
/// With and optional layer
/// .class1.class2::layer
#[derive(Debug)]
pub struct Intersection(Vec<Selector>, Option<Layer>);

#[derive(Debug)]
pub enum Selector {
	SubSelector(SubSelector),
	ChildRelation(ChildRelation),
}

/// A >[link] B
/// Only matches if B is child of A with optional link condition
#[derive(Debug)]
pub struct ChildRelation(Box<SubSelector>, Box<SubSelector>, Option<Link>);
// Use box to reduce size of Selector 96 vs 240 bytes

#[derive(Debug)]
pub struct Link(Property, BinaryOp, Expression);

#[derive(Debug)]
pub enum Property {
	Index,
	Role,
}

/// Operations permitted in link specifier
const LINK_OPERATIONS: [BinaryOp; 3] = [BinaryOp::Eq, BinaryOp::NotEq, BinaryOp::ListMembership];

#[derive(Debug)]
pub enum SubSelector {
	Object(Object, Option<Zoom>, Vec<Condition>, Option<PseudoClass>),
	Class(String),
	PseudoClass(String),
	NotClass(String),
	NotPseudoClass(String),
	Setting(String),
}

type Layer = String;

type PseudoClass = String;

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
	Expression(Expression),
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

pub enum Expression {
	Number(f64),
	Identifier(String),
	String(String),
	Function(Function),
	Binary(Box<Expression>, BinaryOp, Box<Expression>),
	Unary(Box<Expression>, UnaryOp),
}

impl Display for Expression {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Number(n) => write!(f, "{n}"),
			Self::Identifier(name) => write!(f, "{name}"),
			Self::String(string) => write!(f, "\"{string}\""),
			Self::Function(function) => write!(f, "{function}"),
			Self::Binary(a, op, b) => write!(f, "({a} {op} {b})"),
			Self::Unary(expr, op) => write!(f, "{op}{expr}"),
		}
	}
}

impl Debug for Expression {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{self}")
	}
}

#[derive(Debug)]
pub enum Condition {
	Binary(String, BinaryOp, Expression),
	Not(Box<Condition>),
	Truthy(String),
	Falsy(String),
	Tag(String),
	Function(Function),
	Regex(String, String),
}

#[derive(Debug)]
pub struct Function(String, Vec<Expression>);

impl Display for Function {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}(", self.0)?;
		for (i, arg) in self.1.iter().enumerate() {
			if i > 0 {
				write!(f, ", ")?;
			}
			write!(f, "{}", arg)?;
		}
		write!(f, ")")
	}
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
		let test_files = [
			(include_str!("test/simple.mapcss"), "simple.css"),
			(include_str!("test/josm.mapcss"), "josm.css"),
			(include_str!("test/apple.mapcss"), "apple.css"),
		];
		for (src, file_name) in test_files {
			println!("Parsing {file_name:?}");

			let token_iter = Token::lexer(src).spanned().filter_map(|(tok, span)| match tok {
				Ok(tok) => Some((tok, span.into())),
				_ => None,
			});

			let start = std::time::Instant::now();
			let _tokens = token_iter.clone().collect::<Vec<_>>();
			println!("Lexed in {:?}", start.elapsed());

			let token_stream = Stream::from_iter(token_iter).spanned(SimpleSpan::from(src.len()..src.len()));

			let start = std::time::Instant::now();
			let result = expr_parser().parse(token_stream).into_result();
			println!("Parsed in {:?}", start.elapsed());
			match result {
				Ok(_) => {
					println!("Parsed sucessfully");
				}
				Err(errs) => {
					let src = Source::from(&src);
					for err in errs {
						Report::build(ReportKind::Error, &file_name, err.span().start)
							.with_message(err.to_string())
							.with_label(
								Label::new((&file_name, err.span().into_range()))
									.with_message(err.reason())
									.with_color(Color::Red),
							)
							.finish()
							.eprint((&file_name, src.clone()))
							.unwrap();
					}
				}
			}
		}
	}
}
