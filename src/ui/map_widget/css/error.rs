use std::{collections::HashSet, fmt::Display};

use ariadne::{Color, Config, Label, Report, ReportKind, Source};
use log::warn;
use mappar_css::{EvalError, SimpleSpan, Specifier};

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Illegal specifier of type '{}' for '{}'.", .1.get_type(), .0)]
	IllegalSpecifier(String, Specifier),

	#[error("Unknown property: {0}")]
	UnknownProperty(String),

	#[error("Couldn't evaluate expression: {}", .0.0)]
	EvaluationFailed(EvalError),
}

pub struct Reporting {
	file_name: String,
	source: Source,
	messages: HashSet<SimpleSpan>,
}

impl Reporting {
	pub fn new(file_name: String, file_source: String) -> Self {
		Self {
			file_name,
			source: Source::from(file_source),
			messages: HashSet::new(),
		}
	}

	pub fn report(&mut self, message: impl Display, span: SimpleSpan) {
		if !self.messages.insert(span) {
			return;
		}

		let report = Report::build(ReportKind::Error, (&self.file_name, span.into_range()))
			.with_config(Config::new().with_color(false))
			.with_message("Invalid CSS")
			.with_label(
				Label::new((&self.file_name, span.into_range()))
					.with_message(message.to_string())
					.with_color(Color::Red),
			)
			.finish();

		let mut output = Vec::<u8>::new();
		report
			.write_for_stdout((&self.file_name, self.source.clone()), &mut output)
			.unwrap();
		let output_string = String::from_utf8_lossy(&output).into_owned();
		warn!("{output_string}");
	}
}
