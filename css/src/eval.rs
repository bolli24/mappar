use std::fmt::{Debug, Display};

use crate::{BinaryOp, Function, UnaryOp};
use crate::{SimpleSpan, Spanned};

// TODO: replace Number etc with Value
#[derive(Clone)]
pub enum Expression {
	Number(f64),
	Identifier(String),
	String(String),
	Bool(bool),
	Function(Function),
	Binary(Box<Spanned<Expression>>, BinaryOp, Box<Spanned<Expression>>),
	Unary(Box<Spanned<Expression>>, UnaryOp),
}

/// Return value of expressions
#[derive(Debug, Clone)]
pub enum Value {
	Number(f64),
	Identifier(String),
	String(String),
	Bool(bool),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ValueType {
	Number,
	Identifier,
	String,
	Bool,
}

impl Value {
	pub fn get_type(&self) -> ValueType {
		match self {
			Self::Number(_) => ValueType::Number,
			Self::Identifier(_) => ValueType::Identifier,
			Self::String(_) => ValueType::String,
			Self::Bool(_) => ValueType::Bool,
		}
	}

	pub fn unrwap_number(self) -> f64 {
		if let Value::Number(number) = self {
			number
		} else {
			panic!("Expected value to be number: {self:?}");
		}
	}

	pub fn unwrap_identifier(self) -> String {
		if let Value::Identifier(identifier) = self {
			identifier
		} else {
			panic!("Expected value to be identifier: {self:?}");
		}
	}

	pub fn unwrap_string(self) -> String {
		if let Value::String(string) = self {
			string
		} else {
			panic!("Expected value to be string: {self:?}");
		}
	}

	pub fn unwrap_bool(self) -> bool {
		if let Value::Bool(boolean) = self {
			boolean
		} else {
			panic!("Expected value to be bool: {self:?}");
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
	Named,
	Sizes,
	Color,
	Url,
	Expression(ExpressionType),
	Function(ExpressionType),
}

impl Display for Type {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let type_str = match self {
			Type::Named => "named",
			Type::Sizes => "sizes",
			Type::Color => "color",
			Type::Url => "URL",
			Type::Expression(expression_type) | Type::Function(expression_type) => match expression_type {
				ExpressionType::Number => "number",
				ExpressionType::String => "string",
				ExpressionType::Bool => "boolean",
				ExpressionType::Regex => "regular expression",
				ExpressionType::Unknown => "unknown",
			},
		};

		write!(f, "{type_str}")
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpressionType {
	Number,
	String,
	Bool,
	Regex,
	Unknown,
}

#[derive(Debug, Clone)]
pub struct Error(pub Reason, pub SimpleSpan);

#[derive(thiserror::Error, Debug, Clone)]
pub enum Reason {}

impl Expression {
	pub fn eval(&self) -> Result<Value, Error> {
		let value = match self {
			Expression::Number(number) => Value::Number(*number),
			Expression::String(string) => Value::String(string.clone()),
			Expression::Bool(bool) => Value::Bool(*bool),
			Expression::Identifier(identifier) => Value::Identifier(identifier.clone()),
			_ => todo!(),
		};
		Ok(value)
	}

	pub fn get_type(&self) -> ExpressionType {
		match self {
			Self::Number(_) => ExpressionType::Number,
			Self::Identifier(_) => ExpressionType::Unknown,
			Self::String(_) => ExpressionType::String,
			Self::Bool(_) => ExpressionType::Bool,
			Self::Function(function) => function.get_return_type(),
			Self::Binary(a, op, b) => {
				let a_type = a.0.get_type();
				let b_type = b.0.get_type();
				if a_type == ExpressionType::Unknown || b_type == ExpressionType::Unknown {
					return ExpressionType::Unknown;
				}
				match op {
					BinaryOp::Eq | BinaryOp::NotEq => {
						if a_type == b_type {
							ExpressionType::Bool
						} else {
							ExpressionType::Unknown
						}
					}
					BinaryOp::ListMembership => todo!(),
					BinaryOp::RegexMatch => {
						if a_type == ExpressionType::String && b_type == ExpressionType::Regex {
							ExpressionType::Bool
						} else {
							ExpressionType::Unknown
						}
					}
					BinaryOp::Le | BinaryOp::Ge | BinaryOp::Lt | BinaryOp::Gt => {
						if a_type == ExpressionType::Number && b_type == ExpressionType::Number {
							ExpressionType::Bool
						} else {
							ExpressionType::Unknown
						}
					}
					BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
						if a_type == ExpressionType::Number && b_type == ExpressionType::Number {
							ExpressionType::Number
						} else {
							ExpressionType::Unknown
						}
					}
				}
			}
			Self::Unary(value, op) => {
				let value_type = value.0.get_type();
				match op {
					UnaryOp::Not if value_type == ExpressionType::Bool => ExpressionType::Bool,
					UnaryOp::Neg if value_type == ExpressionType::Number => ExpressionType::Number,
					_ => ExpressionType::Unknown,
				}
			}
		}
	}
}

impl Display for Expression {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Number(n) => write!(f, "{n}"),
			Self::Identifier(name) => write!(f, "{name}"),
			Self::String(string) => write!(f, "\"{string}\""),
			Self::Bool(bool) => write!(f, "{bool}"),
			Self::Function(function) => write!(f, "{function}"),
			Self::Binary(a, op, b) => write!(f, "({} {} {})", a.0, op, b.0),
			Self::Unary(expr, op) => write!(f, "{op}{}", expr.0),
		}
	}
}

impl Debug for Expression {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{self}")
	}
}
