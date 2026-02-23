use crate::Spanned;
use crate::{BinaryOp, Function, UnaryOp};
use ecolor::Color32;
use std::fmt::{Debug, Display};

// TODO: replace Number etc with Value
#[derive(Clone)]
pub enum Expression {
	Number(f64),
	String(String),
	Color(Color32),
	Bool(bool),
	Function(Function),
	Binary(Box<Spanned<Self>>, BinaryOp, Box<Spanned<Self>>),
	Unary(Box<Spanned<Self>>, UnaryOp),
}

/// Return value of expressions
#[derive(Debug, Clone)]
pub enum Value {
	Number(f64),
	String(String),
	Color(Color32),
	Bool(bool),
}

impl Value {}

impl TryFrom<Value> for f64 {
	type Error = Value;

	fn try_from(value: Value) -> Result<Self, Self::Error> {
		if let Value::Number(number) = value {
			Ok(number)
		} else {
			Err(value)
		}
	}
}

impl TryFrom<Value> for String {
	type Error = Value;

	fn try_from(value: Value) -> Result<Self, Self::Error> {
		if let Value::String(string) = value {
			Ok(string)
		} else {
			Err(value)
		}
	}
}

impl TryFrom<Value> for Color32 {
	type Error = Value;

	fn try_from(value: Value) -> Result<Self, Self::Error> {
		if let Value::Color(color) = value {
			Ok(color)
		} else {
			Err(value)
		}
	}
}

impl TryFrom<Value> for bool {
	type Error = Value;

	fn try_from(value: Value) -> Result<Self, Self::Error> {
		if let Value::Bool(boolean) = value {
			Ok(boolean)
		} else {
			Err(value)
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
	Rotation,
}

impl Display for Type {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let type_str = match self {
			Self::Named => "named",
			Self::Sizes => "sizes",
			Self::Color => "color",
			Self::Url => "URL",
			Self::Expression(expression_type) | Self::Function(expression_type) => match expression_type {
				ExpressionType::Number => "number",
				ExpressionType::String => "string",
				ExpressionType::Color => "color",
				ExpressionType::Bool => "boolean",
				ExpressionType::Regex => "regular expression",
				ExpressionType::Unknown => "unknown",
			},
			Self::Rotation => "rotation",
		};

		write!(f, "{type_str}")
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpressionType {
	Number,
	String,
	Color,
	Bool,
	Regex,
	Unknown,
}

impl Expression {
	#[must_use]
	pub fn get_type(&self) -> ExpressionType {
		match self {
			Self::Number(_) => ExpressionType::Number,
			Self::String(_) => ExpressionType::String,
			Self::Color(_) => ExpressionType::Color,
			Self::Bool(_) => ExpressionType::Bool,
			Self::Function(function) => function.get_return_type(),
			Self::Binary(a, op, b) => {
				let Spanned(a_expr, _) = a.as_ref();
				let Spanned(b_expr, _) = b.as_ref();
				let a_type = a_expr.get_type();
				let b_type = b_expr.get_type();
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
					BinaryOp::StartsWith => {
						if a_type == ExpressionType::String && b_type == ExpressionType::String {
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
				let Spanned(value_expr, _) = value.as_ref();
				let value_type = value_expr.get_type();
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
			Self::String(string) => write!(f, "\"{string}\""),
			Self::Color(color) => write!(f, "{color:?}"),
			Self::Bool(bool) => write!(f, "{bool}"),
			Self::Function(function) => write!(f, "{function}"),
			Self::Binary(a, op, b) => {
				let Spanned(a_expr, _) = a.as_ref();
				let Spanned(b_expr, _) = b.as_ref();
				write!(f, "({} {} {})", a_expr, op, b_expr)
			}
			Self::Unary(expr, op) => {
				let Spanned(expr_inner, _) = expr.as_ref();
				write!(f, "{op}{}", expr_inner)
			}
		}
	}
}

impl Debug for Expression {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{self}")
	}
}
