use std::{collections::HashSet, fmt::Display};

use ariadne::{Color, Config, Label, Report, ReportKind, Source};
use log::warn;
use mappar_css::{SimpleSpan, Specifier};

use crate::ui::map_widget::css::eval::EvalError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Illegal specifier of type '{}' for '{}'.", .1.get_type(), .0)]
	InvalidSpecifierType(String, Specifier),

	#[error("Invalid specifier variant '{}' for '{}'. Must be one of: {}", .0, .1, .2.join(", "))]
	InvalidSpecifierVariant(String, String, &'static [&'static str]),

	#[error("Unknown property: {0}")]
	UnknownProperty(String),

	#[error(transparent)]
	EvaluationFailed(#[from] EvalError),
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
		puffin::profile_function!();
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
			.expect("writing to Vec<u8>");
		let output_string = String::from_utf8_lossy(&output).into_owned();
		warn!("{output_string}");
	}
}
