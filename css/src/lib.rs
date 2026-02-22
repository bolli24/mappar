#![allow(dead_code)]
use crate::lexer::Token;
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::input::{Stream, ValueInput};
use chumsky::prelude::*;
use chumsky::recursive::Indirect;
use ecolor::Color32;
use logos::Logos;
use std::f64::consts::PI;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::ops::Mul;

pub mod color;
pub mod expression;
pub mod lexer;

pub use chumsky::span::SimpleSpan;
pub use expression::{Expression, ExpressionType, Type, Value, ValueType};

// TODO: better regex

#[derive(Debug, Clone)]
pub struct RuleSet(pub Vec<Styleset>);

#[derive(Debug, Clone, Copy, Default)]
pub struct ParseStats {
	pub lex_errors: usize,
	pub parse_errors: usize,
	pub rules: usize,
	pub selector_groups: usize,
	pub selectors: usize,
	pub declarations: usize,
	pub style_decls: usize,
	pub class_assigns: usize,
}

#[must_use]
pub fn parse_str(src: &str, file_name: &str) -> Option<RuleSet> {
	parse_str_with_stats(src, file_name).0
}

#[must_use]
pub fn parse_str_with_stats(src: &str, file_name: &str) -> (Option<RuleSet>, ParseStats) {
	let mut tokens = Vec::new();
	let mut lex_errors: Vec<Rich<'_, Token<'_>>> = Vec::new();
	for (tok, span) in Token::lexer(src).spanned() {
		let span: SimpleSpan = span.into();
		match tok {
			Ok(tok) => tokens.push((tok, span)),
			Err(err) => lex_errors.push(Rich::custom(span, err.to_string())),
		}
	}
	let mut stats = ParseStats {
		lex_errors: lex_errors.len(),
		..ParseStats::default()
	};
	if !lex_errors.is_empty() {
		let source = Source::from(src);
		for err in lex_errors {
			Report::build(ReportKind::Error, (&file_name, err.span().into_range()))
				.with_message(err.to_string())
				.with_label(
					Label::new((&file_name, err.span().into_range()))
						.with_message(err.reason())
						.with_color(Color::Red),
				)
				.finish()
				.eprint((&file_name, source.clone()))
				.expect("writing to stderr");
		}
		return (None, stats);
	}

	let token_stream = Stream::from_iter(tokens).map((0..src.len()).into(), |(t, s): (_, _)| (t, s));
	let (result, errs) = expr_parser().parse(token_stream).into_output_errors();
	stats.parse_errors = errs.len();
	if let Some(rule_set) = &result {
		stats.rules = rule_set.0.len();
		for styleset in &rule_set.0 {
			stats.declarations += styleset.declarations.len();
			stats.selector_groups += styleset.selectors.0.len();
			for intersection in &styleset.selectors.0 {
				stats.selectors += intersection.0.len();
			}
			for declaration in &styleset.declarations {
				match declaration {
					Declaration::Style(_) => stats.style_decls += 1,
					Declaration::ClassAssign(_) => stats.class_assigns += 1,
				}
			}
		}
	}

	if errs.is_empty() {
		return (result, stats);
	}

	let source = Source::from(src);
	for err in errs {
		Report::build(ReportKind::Error, (&file_name, err.span().into_range()))
			.with_message(err.to_string())
			.with_label(
				Label::new((&file_name, err.span().into_range()))
					.with_message(err.reason())
					.with_color(Color::Red),
			)
			.finish()
			.eprint((&file_name, source.clone()))
			.expect("writing to stderr");
	}
	(None, stats)
}

fn expr_parser<'a, I>() -> impl Parser<'a, I, RuleSet, extra::Err<Rich<'a, Token<'a>>>> + Clone
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

	let quoted = select! { Token::Quoted(s) => s};

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
		Token::Op("^=") => BinaryOp::StartsWith,
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

	let mut expression: Recursive<Indirect<'_, '_, _, Spanned<Expression>, _>> = Recursive::<_>::declare();

	let function = identifier
		.then(
			expression
				.clone()
				.separated_by(just(Token::Comma).padded_by(opt_spaces.clone()))
				.collect::<Vec<_>>()
				.padded_by(opt_spaces.clone())
				.delimited_by(just(Token::ParenOpen), just(Token::ParenClose)),
		)
		.map(|(name, args)| Function(name, args));

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
			Token::Quoted(s) => Expression::String(s),
			Token::Color(color) => Expression::Color(color),
		},
		number.map(Expression::Number),
		expression.clone().map(|expr| expr.0).delimited_by(
			just(Token::ParenOpen).padded_by(opt_spaces.clone()),
			just(Token::ParenClose).padded_by(opt_spaces.clone()),
		),
	))
	.map_with(|atom: Expression, extras| Spanned(atom, extras.span()));

	let unary_expression = unary.repeated().foldr_with(atom, |op, expr, extras| {
		Spanned(Expression::Unary(Box::new(expr), op), extras.span())
	});

	let factor = unary_expression.clone().then_ignore(opt_spaces.clone()).foldl_with(
		binary
			.filter(|op| matches!(op, BinaryOp::Mul | BinaryOp::Div))
			.then_ignore(opt_spaces.clone())
			.then(unary_expression.clone())
			.repeated(),
		|a: Spanned<Expression>, (op, b): (BinaryOp, Spanned<Expression>), extras| {
			Spanned(Expression::Binary(Box::new(a), op, Box::new(b)), extras.span())
		},
	);

	let term = factor.clone().then_ignore(opt_spaces.clone()).foldl_with(
		binary
			.filter(|op| matches!(op, BinaryOp::Add | BinaryOp::Sub))
			.then_ignore(opt_spaces.clone())
			.then(factor.clone())
			.repeated(),
		|a: Spanned<Expression>, (op, b): (BinaryOp, Spanned<Expression>), extras| {
			Spanned(Expression::Binary(Box::new(a), op, Box::new(b)), extras.span())
		},
	);

	let comparison = term.clone().then_ignore(opt_spaces.clone()).foldl_with(
		binary
			.filter(|op| {
				matches!(
					op,
					BinaryOp::Eq
						| BinaryOp::NotEq | BinaryOp::Le
						| BinaryOp::Ge | BinaryOp::Lt
						| BinaryOp::Gt | BinaryOp::StartsWith
				)
			})
			.then_ignore(opt_spaces.clone())
			.then(term.clone())
			.repeated(),
		|a: Spanned<Expression>, (op, b): (BinaryOp, Spanned<Expression>), extras| {
			Spanned(Expression::Binary(Box::new(a), op, Box::new(b)), extras.span())
		},
	);

	/*
	   let atom = int
	   .or(expr.delimited_by(just('('), just(')')))
	   .or(call)
	   .or(ident.map(Expr::Var));
	*/

	expression.define(comparison.labelled("expression"));

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
		Token::Unit(unit) => Unit::try_from(unit).expect("token already matched as Unit"),
	};

	let size = number.then(unit.or_not()).map(|(number, unit)| Size { number, unit });

	let sizes = size
		.separated_by(just(Token::Comma).padded_by(opt_spaces.clone()))
		.at_least(1)
		.collect::<Vec<Size>>()
		.filter(|sizes| sizes.len() > 1 || sizes[0].unit.is_some()); // A single size without a unit is a number

	let rotation_unit = select! { Token::RotationUnit(unit) => RotationUnit::try_from(unit).expect("token already matched as RotationUnit") };

	let rotation = number.then(rotation_unit).map(|(number, unit)| unit * number);

	let specifier = choice((
		labelled_color.map(|(label, color)| Specifier::Color(color, label)),
		sizes.map(Specifier::Sizes),
		rotation.map(Specifier::Rotation),
		expression.clone().map(|expression| match expression {
			Spanned(Expression::Identifier(name), _) => Specifier::Named(name),
			_ => Specifier::Expression(expression),
		}),
	))
	.map_with(|specifier, extras| (specifier, extras.span()))
	.labelled("specifier");

	let key_specifier = key
		.then_ignore(just(Token::Colon).padded_by(opt_spaces.clone()))
		.then(specifier)
		.map_with(|(key, specifier), extras| {
			Declaration::Style(Spanned(
				CssStyle {
					key,
					specifier: specifier.into(),
				},
				extras.span(),
			))
		});

	let class_assign = just(Token::Identifier("set"))
		.ignore_then(req_space.clone())
		.ignore_then(just(Token::Dot).ignored().or(empty().ignored()))
		.ignore_then(identifier)
		.map(|name| Declaration::ClassAssign(name));

	let style = choice((key_specifier, class_assign)).recover_with(skip_then_retry_until(
		any().ignored(),
		just(Token::BraceClose).ignored(),
	));

	let styleset = style
		.padded_by(opt_spaces.clone())
		.clone()
		.separated_by(just(Token::Semicolon).padded_by(opt_spaces.clone()))
		.allow_trailing()
		.collect::<Vec<Declaration>>();

	let declaration = styleset
		.or(empty().map(|()| Vec::new())) // TODO: do we need this?
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

	let regex = select!(Token::Regex(regex) => regex);

	let binary_condition = tag
		.clone()
		.then_ignore(opt_spaces.clone())
		.then(binary)
		.then_ignore(opt_spaces.clone())
		.then(expression.clone());

	let regex_condition = tag
		.clone()
		.then_ignore(opt_spaces.clone())
		.then(select! {
			Token::Op("=~") => false,
			Token::Op("!~") => true,
		})
		.then_ignore(opt_spaces.clone())
		.then(regex)
		.map(|((name, is_negated), regex)| {
			if is_negated {
				Condition::Not(Box::new(Condition::Regex(name, regex)))
			} else {
				Condition::Regex(name, regex)
			}
		});

	let expression_condition =
		term.clone()
			.then_ignore(opt_spaces.clone())
			.then(binary.filter(|op| {
				matches!(
					op,
					BinaryOp::Eq
						| BinaryOp::NotEq | BinaryOp::Le
						| BinaryOp::Ge | BinaryOp::Lt
						| BinaryOp::Gt | BinaryOp::StartsWith
						| BinaryOp::ListMembership
				)
			}))
			.then_ignore(opt_spaces.clone())
			.then(term.clone())
			.map(|((a, op), b)| Condition::Binary(a, op, b));

	// TODO: at not falsy and not truthy: !building?!  !building?
	let condition = recursive(|condition| {
		choice((
			regex_condition,
			regex.map(Condition::TagRegex),
			expression_condition.clone(),
			function.clone().map(Condition::Function), // function
			binary_condition
				.clone() // tag binary expression
				.map_with(|((tag, op), value), extras| {
					let span = extras.span();
					Condition::Binary(Spanned(Expression::Identifier(tag), span), op, value)
				}),
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

	let tests = test.clone().repeated().collect::<Vec<_>>();
	let tests_required = test.repeated().at_least(1).collect::<Vec<_>>();

	let maybe_layer = just(Token::Colon)
		.ignore_then(just(Token::Colon))
		.ignore_then(identifier)
		.or_not();

	// object       ::= "node" | "way" | "relation" | "area" | "canvas" | "*" | ;
	let object = any()
		.filter(|t: &Token| t.try_into_object().is_some())
		.map(|t| t.try_into_object().expect("filtered by is_some"))
		.labelled("object (node, way, relation, area, canvas, *, meta)");

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
		.try_map(|zoom, span| zoom.map_err(|()| Rich::custom(span, "Invalid zoom specifier: {s}")))
		.or_not();

	let setting = just(Token::Identifier("setting"))
		.ignore_then(just(Token::Colon))
		.ignore_then(just(Token::Colon))
		.ignore_then(select! {
			Token::Identifier(s) => s
		})
		.map(|s| SubSelector::Setting(s.to_string()));

	let setting_group = just(Token::Identifier("settings"))
		.ignore_then(just(Token::Colon))
		.ignore_then(just(Token::Colon))
		.ignore_then(select! {
			Token::Identifier(s) => s
		})
		.map(|s| SubSelector::SettingGroup(s.to_string()));

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
		setting_group,
		tests_required.map(SubSelector::Tests),
	))
	.labelled("subselector");

	let link_property = identifier.try_map(|name, span| match name.as_str() {
		"index" => Ok(LinkProperty::Index),
		"role" => Ok(LinkProperty::Role),
		_ => Err(Rich::custom(
			span,
			format!("Invalid property {name} in link condition. Must be either 'index' or 'role'."),
		)),
	});

	let link = link_property
		.then_ignore(opt_spaces.clone())
		.then(binary)
		.then_ignore(opt_spaces.clone())
		.then(expression)
		.padded_by(opt_spaces.clone())
		.delimited_by(just(Token::BracketOpen), just(Token::BracketClose))
		.try_map(|((a, op), b), span| {
			if LINK_OPERATIONS.contains(&op) {
				Ok(Link(a, op, b.0))
			} else {
				Err(Rich::custom(span, format!("Invalid operator {op}")))
			}
		})
		.or_not();

	let child_relation = sub_selector
		.clone()
		.then_ignore(opt_spaces.clone())
		.then_ignore(just(Token::Op(">")))
		.then(link.clone())
		.then_ignore(opt_spaces.clone())
		.then(sub_selector.clone())
		.map(|((a, link), b)| ChildRelation(Box::new(a), Box::new(b), link));

	let overlap_relation = sub_selector
		.clone()
		.then_ignore(opt_spaces.clone())
		.then_ignore(just(Token::Overlap))
		.then_ignore(opt_spaces.clone())
		.then(sub_selector.clone())
		.map(|(a, b)| OverlapRelation(Box::new(a), Box::new(b)));

	let parent_relation = sub_selector
		.clone()
		.then_ignore(opt_spaces.clone())
		.then_ignore(just(Token::Op("<")))
		.then(link)
		.then_ignore(opt_spaces.clone())
		.then(sub_selector.clone())
		.map(|((a, link), b)| ParentRelation(Box::new(a), Box::new(b), link));

	let set_operator = select! {
		Token::ElementOf => SetOperator::ElementOf,
		Token::SubsetOrEqual => SetOperator::SubsetOrEqual,
		Token::SupersetOrEqual => SetOperator::SupersetOrEqual,
		Token::NotSubsetOrEqual => SetOperator::NotSubsetOrEqual,
		Token::NotSupersetOrEqual => SetOperator::NotSupersetOrEqual,
	};

	let set_relation = sub_selector
		.clone()
		.then_ignore(opt_spaces.clone())
		.then(set_operator)
		.then_ignore(opt_spaces.clone())
		.then(sub_selector.clone())
		.map(|((a, op), b)| SetRelation(Box::new(a), Box::new(b), op));

	//	selector     ::= subselector | child;
	let selector = choice((
		child_relation.map(Selector::ChildRelation),
		parent_relation.map(Selector::ParentRelation),
		overlap_relation.map(Selector::OverlapRelation),
		set_relation.map(Selector::SetRelation),
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
		.recover_with(via_parser(none_of(Token::BraceOpen).repeated().map(|()| Vec::new())))
		.map(Union);

	let selectors = union_selector.labelled("selectors");

	let rule = selectors.then(declaration).map(|(selectors, styles)| Styleset {
		selectors,
		declarations: styles,
	});

	let rule_set = rule
		.separated_by(opt_spaces.clone())
		.collect()
		.padded_by(opt_spaces.clone());

	rule_set.map(RuleSet)
}

#[derive(Debug, Clone)]
pub struct Spanned<T: Debug + Clone>(pub T, pub SimpleSpan);

impl<T: Debug + Clone> From<(T, SimpleSpan)> for Spanned<T> {
	fn from(value: (T, SimpleSpan)) -> Self {
		Self(value.0, value.1)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
	Eq,
	NotEq,
	ListMembership,
	RegexMatch,
	StartsWith,
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
			Self::Eq => write!(f, "="),
			Self::NotEq => write!(f, "!="),
			Self::ListMembership => write!(f, "~="),
			Self::RegexMatch => write!(f, "=~"),
			Self::StartsWith => write!(f, "^="),
			Self::Le => write!(f, "<="),
			Self::Ge => write!(f, ">="),
			Self::Lt => write!(f, "<"),
			Self::Gt => write!(f, ">"),
			Self::Mul => write!(f, "*"),
			Self::Add => write!(f, "+"),
			Self::Sub => write!(f, "-"),
			Self::Div => write!(f, "/"),
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
	Not,
	Neg,
}

impl Display for UnaryOp {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Not => write!(f, "!"),
			Self::Neg => write!(f, "-"),
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub enum Object {
	Node,
	Way,
	Relation,
	Area,
	Canvas,
	All,
	Meta,
}

#[derive(Debug, Clone, Copy)]
pub struct Zoom(pub u8, pub u8);

impl TryFrom<&str> for Zoom {
	type Error = ();

	fn try_from(value: &str) -> Result<Self, Self::Error> {
		match value.split('-').collect::<Vec<_>>().as_slice() {
			[z] => {
				let z = z.parse::<u8>().map_err(|_| ())?;
				Ok(Self(z, z))
			}
			[z1, ""] => {
				let z1 = z1.parse::<u8>().map_err(|_| ())?;
				Ok(Self(z1, u8::MAX))
			}
			["", z2] => {
				let z2 = z2.parse::<u8>().map_err(|_| ())?;
				Ok(Self(0, z2))
			}
			[z1, z2] => {
				let z1 = z1.parse::<u8>().map_err(|_| ())?;
				let z2 = z2.parse::<u8>().map_err(|_| ())?;
				Ok(Self(z1, z2))
			}
			_ => Err(()),
		}
	}
}

#[derive(Debug, Clone)]
pub struct Styleset {
	pub selectors: Selectors,
	pub declarations: Vec<Declaration>,
}

pub type Selectors = Union;

/// Applies to all instances with any of the subselectors
/// .class1, .class2
#[derive(Debug, Clone)]
pub struct Union(pub Vec<Intersection>);

impl Hash for Union {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		format!("{self:?}").hash(state);
	}
}

impl PartialEq for Union {
	fn eq(&self, other: &Self) -> bool {
		format!("{self:?}") == format!("{other:?}")
	}
}

impl Eq for Union {}

/// Applies to all instances with all the subselectors
/// `.class1.class2` or `.class1 .class2`
/// With and optional layer
/// `.class1.class2::layer`
#[derive(Debug, Clone)]
pub struct Intersection(pub Vec<Selector>, pub Option<Layer>);

#[derive(Debug, Clone)]
pub enum Selector {
	SubSelector(SubSelector),
	ChildRelation(ChildRelation),
	OverlapRelation(OverlapRelation),
	ParentRelation(ParentRelation),
	SetRelation(SetRelation),
}

/// A >[link] B
/// Only matches if B is child of A with optional link condition
#[derive(Debug, Clone)]
pub struct ChildRelation(pub Box<SubSelector>, pub Box<SubSelector>, pub Option<Link>);
// Uses box to reduce size of Selector (72 vs 168 bytes)

#[derive(Debug, Clone)]
pub struct OverlapRelation(pub Box<SubSelector>, pub Box<SubSelector>);

#[derive(Debug, Clone)]
pub struct ParentRelation(pub Box<SubSelector>, pub Box<SubSelector>, pub Option<Link>);

#[derive(Debug, Clone, Copy)]
pub enum SetOperator {
	ElementOf,
	SubsetOrEqual,
	SupersetOrEqual,
	NotSubsetOrEqual,
	NotSupersetOrEqual,
}

#[derive(Debug, Clone)]
pub struct SetRelation(pub Box<SubSelector>, pub Box<SubSelector>, pub SetOperator);

#[derive(Debug, Clone)]
pub struct Link(LinkProperty, BinaryOp, Expression);

#[derive(Debug, Clone, Copy)]
pub enum LinkProperty {
	Index,
	Role,
}

/// Operations permitted in link specifier
const LINK_OPERATIONS: [BinaryOp; 3] = [BinaryOp::Eq, BinaryOp::NotEq, BinaryOp::ListMembership];

#[derive(Debug, Clone)]
pub enum SubSelector {
	Object(Object, Option<Zoom>, Vec<Condition>, Option<PseudoClass>),
	Class(String),
	PseudoClass(String),
	NotClass(String),
	NotPseudoClass(String),
	Setting(String),
	SettingGroup(String),
	Tests(Vec<Condition>),
}

impl SubSelector {
	#[must_use]
	pub fn pseudo_class(&self) -> Option<&str> {
		match &self {
			Self::Object(_, _, _, pseudo_class) => pseudo_class.as_ref().map(|s| s.as_str()),
			Self::PseudoClass(pseudo_class) => Some(pseudo_class),
			_ => None,
		}
	}
}

type Layer = String;

type PseudoClass = String;

#[derive(Debug, Clone)]
pub enum Declaration {
	Style(Spanned<CssStyle>),
	ClassAssign(String),
}

#[derive(Debug, Clone)]
pub struct CssStyle {
	pub key: String,
	pub specifier: Spanned<Specifier>,
}

#[derive(Debug, Clone)]
pub enum Specifier {
	Named(String),
	Sizes(Vec<Size>),
	Color(Color32, Option<String>),
	Url(String),
	Expression(Spanned<Expression>),
	Function(Function),
	Rotation(f64),
}

impl Specifier {
	#[must_use]
	pub fn get_type(&self) -> Type {
		match self {
			Self::Named(_) => Type::Named,
			Self::Sizes(_) => Type::Sizes,
			Self::Color(_, _) => Type::Color,
			Self::Url(_) => Type::Url,
			Self::Expression(expression) => Type::Expression(expression.0.get_type()),
			Self::Function(function) => Type::Expression(function.get_return_type()),
			Self::Rotation(_) => Type::Rotation,
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Size {
	number: f64,
	unit: Option<Unit>,
}

#[derive(Debug, Clone, Copy)]
pub enum Unit {
	Px,
	Pt,
	Percent,
}

impl TryFrom<&str> for Unit {
	type Error = String;

	fn try_from(s: &str) -> Result<Self, Self::Error> {
		match s {
			"px" => Ok(Self::Px),
			"pt" => Ok(Self::Pt),
			"%" => Ok(Self::Percent),
			_ => Err(format!("Unknown unit: {s}")),
		}
	}
}

impl Mul<f64> for Unit {
	type Output = f64;

	fn mul(self, rhs: f64) -> Self::Output {
		match self {
			Self::Px => rhs,
			Self::Pt => rhs * 4.0 / 3.0,
			Self::Percent => rhs / 100.0,
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub enum RotationUnit {
	Rad,
	Deg,
	Grad,
	Turn,
}

impl TryFrom<&str> for RotationUnit {
	type Error = String;

	fn try_from(s: &str) -> Result<Self, Self::Error> {
		match s {
			"rad" => Ok(Self::Rad),
			"deg" | "°" => Ok(Self::Deg),
			"grad" => Ok(Self::Grad),
			"turn" => Ok(Self::Turn),
			_ => Err(format!("Unknown rotation unit: {s}")),
		}
	}
}

impl Mul<f64> for RotationUnit {
	type Output = f64;

	fn mul(self, rhs: f64) -> Self::Output {
		match self {
			Self::Rad => rhs,
			Self::Deg => rhs.to_radians(),
			Self::Grad => rhs * PI / 200.0,
			Self::Turn => rhs * PI,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Regex(String, bool);

#[derive(Debug, Clone)]
pub enum Condition {
	Binary(Spanned<Expression>, BinaryOp, Spanned<Expression>),
	Not(Box<Self>),
	Truthy(String),
	Falsy(String),
	Tag(String),
	Function(Function),
	Regex(String, Regex),
	TagRegex(Regex),
}

#[derive(Debug, Clone)]
pub struct Function(pub String, pub Vec<Spanned<Expression>>);

impl Function {
	#[must_use]
	pub const fn get_return_type(&self) -> ExpressionType {
		ExpressionType::Unknown
	}
}

impl Display for Function {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}(", self.0)?;
		for (i, arg) in self.1.iter().enumerate() {
			if i > 0 {
				write!(f, ", ")?;
			}
			write!(f, "{}", arg.0)?;
		}
		write!(f, ")")
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::fs::File;
	use std::io::Write;

	/// Parse without printing errors (for testing error cases)
	fn parse_str_silent(src: &str) -> Option<RuleSet> {
		let mut tokens = Vec::new();
		for (tok, span) in Token::lexer(src).spanned() {
			let span: SimpleSpan = span.into();
			match tok {
				Ok(tok) => tokens.push((tok, span)),
				Err(_) => return None,
			}
		}
		let token_stream = Stream::from_iter(tokens).map((0..src.len()).into(), |(t, s): (_, _)| (t, s));
		let (result, errs) = expr_parser().parse(token_stream).into_output_errors();
		if errs.is_empty() { result } else { None }
	}

	#[test]
	fn parse() {
		let test_files = [
			(include_str!("test/simple.mapcss"), "simple.mapcss"),
			(include_str!("test/josm.mapcss"), "josm.mapcss"),
			(include_str!("test/apple.mapcss"), "apple.mapcss"),
			(include_str!("../../assets/styles/default.mapcss"), "default.mapcss"),
		];
		let mut succeeded = 0;
		let mut failed = 0;
		for (src, file_name) in test_files {
			println!("Parsing {file_name:?}");

			let start = std::time::Instant::now();
			let result = parse_str(src, file_name);
			println!("Parsed in {:?}", start.elapsed());

			// Write parsed output to file
			let output_name = file_name.replace(".mapcss", "");
			let output_path = format!("parse_output_{output_name}.txt");
			let file = File::create(&output_path).unwrap();
			let mut writer = std::io::BufWriter::new(file);
			if let Some(rule_set) = &result {
				succeeded += 1;
				writeln!(writer, "Parsed {} rules from {file_name}", rule_set.0.len()).unwrap();
				for (i, styleset) in rule_set.0.iter().enumerate() {
					writeln!(writer, "\nRule {i}:").unwrap();
					writeln!(writer, "  Selectors: {:?}", styleset.selectors).unwrap();
					writeln!(writer, "  Declarations:").unwrap();
					for decl in &styleset.declarations {
						writeln!(writer, "    {:?}", decl).unwrap();
					}
				}
			} else {
				failed += 1;
				writeln!(writer, "Failed to parse {file_name}").unwrap();
			}
		}
		println!("Parse results: {succeeded} succeeded, {failed} failed");
	}

	#[test]
	fn parse_recovers_declaration_block() {
		let src = "way { color: ; width: 2px; }";
		let token_iter = Token::lexer(src).spanned().filter_map(|(tok, span)| match tok {
			Ok(tok) => Some((tok, span.into())),
			_ => None,
		});
		let token_stream = Stream::from_iter(token_iter).map((0..src.len()).into(), |(t, s): (_, _)| (t, s));

		let (output, errors) = expr_parser().parse(token_stream).into_output_errors();
		assert!(output.is_some(), "expected recovered output, errors: {errors:?}");
		assert!(!errors.is_empty(), "expected recovery errors");

		let rule_set = output.unwrap();
		assert_eq!(rule_set.0.len(), 1);
		let styleset = &rule_set.0[0];
		let style_decls: Vec<_> = styleset
			.declarations
			.iter()
			.filter(|decl| matches!(decl, Declaration::Style(_)))
			.collect();
		assert_eq!(style_decls.len(), 1);
		if let Declaration::Style(Spanned(style, _)) = style_decls[0] {
			assert_eq!(style.key, "width");
		} else {
			unreachable!("expected a style declaration");
		}
	}

	#[test]
	fn parse_recovers_declaration_block_nested_delimiters() {
		let src = "way { color rgb(1, 2, 3); width: 2px; }";
		let token_iter = Token::lexer(src).spanned().filter_map(|(tok, span)| match tok {
			Ok(tok) => Some((tok, span.into())),
			_ => None,
		});
		let token_stream = Stream::from_iter(token_iter).map((0..src.len()).into(), |(t, s): (_, _)| (t, s));

		let (output, errors) = expr_parser().parse(token_stream).into_output_errors();
		assert!(output.is_some(), "expected recovered output, errors: {errors:?}");
		assert!(!errors.is_empty(), "expected recovery errors");

		let rule_set = output.unwrap();
		assert_eq!(rule_set.0.len(), 1);
		let styleset = &rule_set.0[0];
		let style_decls: Vec<_> = styleset
			.declarations
			.iter()
			.filter(|decl| matches!(decl, Declaration::Style(_)))
			.collect();
		assert_eq!(style_decls.len(), 1);
		if let Declaration::Style(Spanned(style, _)) = style_decls[0] {
			assert_eq!(style.key, "width");
		} else {
			unreachable!("expected a style declaration");
		}
	}

	#[test]
	fn lexing_error_is_reported() {
		let src = "node ` { color: #fff; }";
		assert!(parse_str_silent(src).is_none());
	}

	#[test]
	fn lexing_error_incomplete_comment() {
		let src = "node /*";
		assert!(parse_str_silent(src).is_none());
	}

	#[test]
	fn lexing_error_invalid_color() {
		let src = "node { color: #ggg; }";
		assert!(parse_str_silent(src).is_none());
	}

	#[test]
	fn lexing_error_multiple() {
		let src = "node ` { color: #ggg; } @";
		assert!(parse_str_silent(src).is_none());
	}

	#[test]
	fn lexing_error_multiple_message() {
		let src = "node ` { color: #ggg; } @";
		assert!(parse_str_silent(src).is_none());
	}
}
