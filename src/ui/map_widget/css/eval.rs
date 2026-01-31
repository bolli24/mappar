use crate::MapElement;
use mappar_css::{Expression, Function, Value};
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum EvalError {
	#[error("Division by zero")]
	DivisionByZero,

	#[error("Unknown function: {0}")]
	UnknownFunction(String),

	#[error("Function '{0}' requires {1} arguments, but got {2}")]
	ArgumentCountMismatch(String, usize, usize),

	#[error("Type mismatch: expected {0}, got {1}")]
	TypeMismatch(String, String),

	#[error("Unknown identifier: {0}")]
	UnknownIdentifier(String),

	#[error("Invalid regex: {0}")]
	InvalidRegex(String),

	#[error("Binary operator '{0}' not supported for types {1} and {2}")]
	InvalidBinaryOp(String, String, String),

	#[error("Unary operator '{0}' not supported for type {1}")]
	InvalidUnaryOp(String, String),

	#[error("Expression not implemented yet: {0}")]
	NotImplemented(String),
}

pub struct EvalContext<'a> {
	pub(crate) element: &'a MapElement,
}

pub trait ExpressionEval {
	fn eval(&self, ctx: &EvalContext) -> Result<Value, EvalError>;
}

impl ExpressionEval for Expression {
	fn eval(&self, ctx: &EvalContext) -> Result<Value, EvalError> {
		let value = match self {
			Self::Number(number) => Value::Number(*number),
			Self::String(string) => Value::String(string.clone()),
			Self::Color(color) => Value::Color(*color),
			Self::Bool(bool) => Value::Bool(*bool),
			Self::Identifier(identifier) => Value::Identifier(identifier.clone()),
			Self::Function(function) => function.eval(ctx)?,
			Self::Binary(_, _, _) | Self::Unary(_, _) => {
				return Err(EvalError::NotImplemented("Binary and Unary expressions".to_string()));
			}
		};
		Ok(value)
	}
}

impl ExpressionEval for Function {
	fn eval(&self, ctx: &EvalContext) -> Result<Value, EvalError> {
		let Function(name, _args) = self;

		match name.as_str() {
			"osm_id" => Ok(Value::String(ctx.element.id.0.to_string())),
			_ => Err(EvalError::UnknownFunction(name.clone())),
		}
	}
}
