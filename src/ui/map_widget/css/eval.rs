use std::cell::RefCell;

use mappar_css::{ExpressionType, Spanned, Specifier, Value};

use super::error::Reporting;

// TODO: handle errors with Span
pub fn try_eval_as(
	specifier: &Spanned<Specifier>,
	expr_type: ExpressionType,
	reporting: &RefCell<Reporting>,
) -> Option<Value> {
	if let Specifier::Expression(expr) = specifier.0 {
		match expr.0.eval() {
			Ok(value) => Some(value),
			Err(err) => {
				reporting.borrow_mut().report(err, expr.1);
				None
			}
		}
	} else {
		reporting.borrow_mut().report(
			format!("Specifier type not supported yet: {}", specifier.0.get_type()),
			*specifier.1,
		);
		None
	}
}
