use crate::map::map_data::{Element, ElementId, MapElement};
use crate::ui::map_widget::style::Style as MapStyle;
use icon::Icon;

use ariadne::{Color, Label, Report, ReportKind, Source};
use egui::mutex::RwLock;
use egui::{Color32, TextureOptions};
use epaint::{TextureId, TextureManager};
use log::{error, info, warn};
use mappar_css::color::get_named_color;
use mappar_css::*;

use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::sync::Arc;

pub mod icon;

// TODO: maybe use sparse set instead of hashmap
pub struct Styles {
	css: CSS,
	styles: HashMap<ElementId, MapStyle>,
	class_assignments: HashMap<Selectors, Vec<String>>,
	icons: Arc<RwLock<HashMap<String, Icon>>>,
	texture_manager: Arc<RwLock<TextureManager>>,
	fallback_texture: TextureId,
	reporting: RefCell<Reporting>,
}

impl Drop for Styles {
	fn drop(&mut self) {
		let mut texture_manager = self.texture_manager.borrow_mut().write();
		for (name, icon) in self.icons.write().drain() {
			info!("Freeing icon texture: {name}");
			texture_manager.free(icon.0);
		}
		if self.fallback_texture != TextureId::Managed(0) {
			texture_manager.free(self.fallback_texture);
		}
	}
}

pub fn get_default_styles(texture_manager: Arc<RwLock<TextureManager>>) -> Option<Styles> {
	info!("Getting default MapCSS styles.");

	const PATH: &str = "./assets/styles/default.mapcss";
	#[cfg(not(target_arch = "wasm32"))]
	let css_string = std::fs::read_to_string(PATH)
		.map_err(|err| {
			error!("Couldn't load default styles ({PATH}): {err:?}");
		})
		.ok()?;
	#[cfg(target_arch = "wasm32")]
	let css_string = include_str!("../../../../assets/styles/default.mapcss").to_string();

	let css = parse_str(&css_string, PATH)?;

	let mut class_assignments: HashMap<Selectors, Vec<String>> = HashMap::new();
	let icons = Arc::new(RwLock::new(HashMap::new()));
	let fallback_texture = get_fallback_texture(&texture_manager);

	let reporting = RefCell::new(Reporting::new(PATH.to_string(), css_string));

	for styleset in &css.0 {
		let mut class_names = Vec::new();
		for rule in &styleset.rules {
			match rule {
				Rule::ClassAssign(class_name) => {
					class_names.push(class_name.clone());
				}
				Rule::Style(style_rule) => {
					if style_rule.0.key == "icon-image" {
						let Spanned(specifier, span) = &style_rule.0.specifier;
						if let Specifier::Expression(Expression::String(file_name)) = specifier {
							load_icon(file_name, &icons, &texture_manager, fallback_texture);
						} else {
							reporting.borrow_mut().report(
								Error::IllegalSpecifier("icon-image".to_string(), specifier.clone()),
								*span,
							);
						};
					}
				}
			}
		}
		if !class_names.is_empty() {
			class_assignments.insert(styleset.selectors.clone(), class_names);
		}
	}

	Some(Styles {
		css,
		styles: HashMap::new(),
		class_assignments,
		icons,
		texture_manager,
		fallback_texture,
		reporting,
	})
}

fn get_fallback_texture(texture_manager: &Arc<RwLock<TextureManager>>) -> TextureId {
	let fallback_image = egui::ColorImage {
		size: [2, 2],
		pixels: vec![
			Color32::from_rgb(255, 0, 255),
			Color32::BLACK,
			Color32::BLACK,
			Color32::from_rgb(255, 0, 255),
		],
	};

	texture_manager.write().alloc(
		"fallback_texture".to_string(),
		egui::ImageData::Color(Arc::new(fallback_image)),
		TextureOptions::NEAREST,
	)
}

impl Styles {
	pub fn get_style(&mut self, element: &MapElement) -> MapStyle {
		/* if !self.styles.contains_key(&element.id) {
			let style = self.create_style(element, map_data);
			self.styles.insert(element.id, style);
		}

		&self.styles[&element.id] */
		self.create_style(element)
	}

	fn create_style(&mut self, element: &MapElement) -> MapStyle {
		let mut style = MapStyle::default();

		let stylesets = matching_rules(&self.css, element);
		// TODO: sort styles by specificity
		for Styleset { rules, .. } in stylesets {
			self.apply_rules(element, &mut style, rules);
		}

		style
	}

	fn apply_rules(&self, element: &MapElement, style: &mut MapStyle, rules: &[Rule]) {
		for rule in rules {
			match rule {
				Rule::Style(style_rule) => {
					self.apply_style_rule(element, style, style_rule);
				}
				Rule::ClassAssign(_) => {}
			}
		}
	}

	fn apply_style_rule(&self, element: &MapElement, style: &mut MapStyle, style_rule: &Spanned<CssStyle>) {
		puffin::profile_function!();
		let specifier = &style_rule.0.specifier.0;
		let span = style_rule.0.specifier.1;
		match style_rule.0.key.as_str() {
			"color" | "colour" => match &style_rule.0.specifier.0 {
				Specifier::Color(color, _name) => {
					style.color = *color;
				}
				Specifier::Named(color_name) => {
					if let Some(color) = get_named_color(color_name) {
						style.color = color
					}
				}
				specifier => self.error(
					Error::IllegalSpecifier(style_rule.0.key.clone(), specifier.clone()),
					span,
				),
			},
			"width" | "size" => match &style_rule.0.specifier.0 {
				Specifier::Expression(Expression::Number(number)) => style.size = *number as f32,
				specifier => self.error(
					Error::IllegalSpecifier(style_rule.0.key.clone(), specifier.clone()),
					span,
				),
			},
			"icon-image" => {
				if let Specifier::Expression(Expression::String(icon_file)) = &style_rule.0.specifier.0 {
					style.icon = self.icons.read().get(icon_file).copied();
				} else {
					self.error(
						Error::IllegalSpecifier(style_rule.0.key.clone(), specifier.clone()),
						span,
					);
				}
			}
			"text" => match &style_rule.0.specifier.0 {
				Specifier::Named(name) => {
					if name == "auto" {
						style.text.text = element.get_auto_text()
					}
				}
				Specifier::Expression(Expression::String(text)) => style.text.text = text.clone(),
				_ => self.error(
					Error::IllegalSpecifier(style_rule.0.key.clone(), specifier.clone()),
					span,
				),
			},
			"font-size" => {
				if let Specifier::Expression(Expression::Number(number)) = &style_rule.0.specifier.0 {
					style.text.font_size = *number as f32;
				} else {
					self.error(
						Error::IllegalSpecifier(style_rule.0.key.clone(), specifier.clone()),
						span,
					)
				}
			}
			"text-color" => match &style_rule.0.specifier.0 {
				Specifier::Color(color, _name) => {
					style.text.color = *color;
				}
				Specifier::Named(color_name) => {
					if let Some(color) = get_named_color(color_name) {
						style.text.color = color
					}
				}
				specifier => self.error(
					Error::IllegalSpecifier(style_rule.0.key.clone(), specifier.clone()),
					span,
				),
			},
			name => self.error(Error::UnknownProperty(name.to_string()), style_rule.1),
		}
	}

	fn error(&self, error: Error, span: SimpleSpan) {
		self.reporting.borrow_mut().report(error, span)
	}
}

#[derive(thiserror::Error, Debug)]
enum Error {
	#[error("Illegal specifier of type {} for {}.", .1.get_type(), .0)]
	IllegalSpecifier(String, Specifier),

	#[error("Unknown property: {0}")]
	UnknownProperty(String),
}

struct Reporting {
	file_name: String,
	source: Source,
	messages: HashSet<SimpleSpan>,
}

fn matching_rules<'a>(css: &'a CSS, element: &'a MapElement) -> impl Iterator<Item = &'a Styleset> {
	puffin::profile_function!();
	css.0.iter().filter(|styleset| matches_rule(styleset, element))
}

fn matches_rule(styleset: &Styleset, element: &MapElement) -> bool {
	let result = &styleset.selectors.0.iter().any(|intersection| {
		intersection
			.0
			.iter()
			.all(|selector| matches_selector(selector, element))
	});

	*result
}

fn matches_selector(selector: &Selector, element: &MapElement) -> bool {
	match selector {
		Selector::SubSelector(selector) => matches_subselector(selector, element),
		Selector::ChildRelation(..) => todo!(),
	}
}

fn matches_subselector(subselector: &SubSelector, element: &MapElement) -> bool {
	match subselector {
		SubSelector::Object(object, _zoom, conditions, _pseudo_class) => {
			matches_object(object, element) && matches_conditions(conditions, element)
		}
		_ => todo!(),
	}
}

fn matches_object(object: &Object, element: &MapElement) -> bool {
	match object {
		Object::Node => matches!(element.element, Element::Node { .. }),
		Object::Way => matches!(element.element, Element::Way { .. }),
		Object::Relation => matches!(element.element, Element::Relation { .. }),
		Object::All => true,
		_ => todo!(),
	}
}

fn matches_conditions(conditions: &[Condition], element: &MapElement) -> bool {
	conditions.iter().all(|condition| match condition {
		Condition::Binary(tag_key, BinaryOp::Eq, Expression::Identifier(tag_value)) => {
			element.get_tag_with_new(tag_key) == Some(tag_value)
		}
		_ => todo!(),
	})
}

fn load_icon(
	file_name: &str,
	icons: &Arc<RwLock<HashMap<String, Icon>>>,
	texture_manager: &Arc<RwLock<TextureManager>>,
	fallback_texture: TextureId,
) {
	let texture_id = icon::try_get_icon_texture(
		file_name,
		texture_manager,
		#[cfg(target_arch = "wasm32")]
		icons,
		#[cfg(target_arch = "wasm32")]
		fallback_texture,
	)
	.unwrap_or_else(|err| {
		error!("Could not load icon '{file_name}': {err}");
		fallback_texture
	});
	icons.write().insert(file_name.to_string(), Icon(texture_id));
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
			// .with_message()
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
