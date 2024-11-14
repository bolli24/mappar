use crate::imagery::imagery_fetcher;
use crate::map::map_data::{Element, MapData, MapElement};
use crate::map::render::ElementId;
use crate::map::style::Style as MapStyle;
use ariadne::{Color, Label, Report, ReportKind, Source};
use bytes::Bytes;
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
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

// TODO: maybe use sparse set instead of hashmap
pub struct Styles {
	css: CSS,
	styles: HashMap<ElementId, MapStyle>,
	class_assignments: HashMap<Selectors, Vec<String>>,
	icons: HashMap<String, Icon>,
	texture_manager: Arc<RwLock<TextureManager>>,
	fallback_texture: TextureId,
	reporting: RefCell<Reporting>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Icon(pub TextureId);

impl Drop for Styles {
	fn drop(&mut self) {
		let mut texture_manager = self.texture_manager.borrow_mut().write();
		for (name, icon) in self.icons.drain() {
			info!("Freeing icon texture: {name}");
			texture_manager.free(icon.0);
		}
		if self.fallback_texture != TextureId::Managed(0) {
			texture_manager.free(self.fallback_texture);
		}
	}
}

// FIXME: Implement proper fallback texture
pub fn get_default_styles(texture_manager: Arc<RwLock<TextureManager>>) -> Option<Styles> {
	info!("Getting default MapCSS styles.");

	const PATH: &str = "./assets/default.mapcss";
	#[cfg(not(target_arch = "wasm32"))]
	let css_string = std::fs::read_to_string(PATH)
		.map_err(|err| {
			error!("Couldn't load default styles ({PATH}): {err:?}");
		})
		.ok()?;
	#[cfg(target_arch = "wasm32")]
	let css_string = include_str!("../assets/default.mapcss").to_string();

	let css = parse_str(&css_string, PATH)?;

	let mut class_assignments: HashMap<Selectors, Vec<String>> = HashMap::new();
	let mut icons = HashMap::new();
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
					if style_rule.key == "icon-image" {
						let Spanned(specifier, span) = &style_rule.specifier;
						if let Specifier::Expression(Expression::String(file_name)) = specifier {
							let texture_id = load_icon(file_name, &texture_manager, fallback_texture);
							icons.insert(file_name.clone(), Icon(texture_id));
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

fn load_icon(file_name: &str, texture_manager: &Arc<RwLock<TextureManager>>, fallback_texture: TextureId) -> TextureId {
	try_get_icon_texture(file_name, texture_manager).unwrap_or_else(|err| {
		error!("Could not load icon '{file_name}': {err}");
		fallback_texture
	})
}

// FIXME: Implement for web.
// TODO: Maybe use async io
fn try_get_icon_texture(
	file_name: &str,
	texture_manager: &Arc<RwLock<TextureManager>>,
) -> Result<TextureId, imagery_fetcher::Error> {
	let file_path = Path::new("./assets").join(file_name);
	// format!("/assets/{file_name}");
	let mut file = File::open(file_path)?;

	let mut image_data = Vec::new();
	file.read_to_end(&mut image_data)?;
	let bytes = Bytes::from(image_data);

	imagery_fetcher::ImageryFetcher::create_texture(texture_manager, format!("icon {file_name}"), &bytes)
}

impl Styles {
	#[cfg(not(target_arch = "wasm32"))]
	pub fn reload_from_file(&mut self) {
		info!("Reloading MapCSS styles.");
		if let Some(styles) = get_default_styles(self.texture_manager.clone()) {
			*self = styles
		} else {
			error!("Unable to reload styles.")
		}
	}

	pub fn get_style(&mut self, element: &MapElement, map_data: &MapData) -> MapStyle {
		/* if !self.styles.contains_key(&element.id) {
			let style = self.create_style(element, map_data);
			self.styles.insert(element.id, style);
		}

		&self.styles[&element.id] */
		self.create_style(element, map_data)
	}

	fn create_style(&mut self, element: &MapElement, map_data: &MapData) -> MapStyle {
		let mut style = MapStyle::default();

		let stylesets = matching_rules(&self.css, map_data.get_element(element.id).unwrap());
		// TODO: sort styles
		for Styleset { rules, .. } in stylesets {
			self.apply_rules(&mut style, rules);
		}

		style
	}

	fn apply_rules(&self, style: &mut MapStyle, rules: &[Rule]) {
		for rule in rules {
			match rule {
				Rule::Style(style_rule) => {
					self.apply_style_rule(style, style_rule);
				}
				Rule::ClassAssign(_) => {}
			}
		}
	}

	fn apply_style_rule(&self, style: &mut MapStyle, style_rule: &CssStyle) {
		puffin::profile_function!();
		let specifier = &style_rule.specifier.0;
		let span = &style_rule.specifier.1;
		match style_rule.key.as_str() {
			"color" | "colour" => match &style_rule.specifier.0 {
				Specifier::Color(color, _name) => {
					style.color = *color;
				}
				Specifier::Named(color_name) => {
					if let Some(color) = get_named_color(color_name) {
						style.color = color
					}
				}
				specifier => self.error(
					Error::IllegalSpecifier(style_rule.key.clone(), specifier.clone()),
					*span,
				),
			},
			"width" | "size" => match &style_rule.specifier.0 {
				Specifier::Expression(Expression::Number(number)) => style.size = *number as f32,
				specifier => self.error(
					Error::IllegalSpecifier(style_rule.key.clone(), specifier.clone()),
					*span,
				),
			},
			"icon-image" => {
				if let Specifier::Expression(Expression::String(icon_file)) = &style_rule.specifier.0 {
					style.icon = self.icons.get(icon_file).copied();
				} else {
					self.error(
						Error::IllegalSpecifier(style_rule.key.clone(), specifier.clone()),
						*span,
					);
				}
			}

			_ => {}
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
}

struct Reporting {
	file_name: String,
	source: Source,
	messages: HashSet<SimpleSpan>,
}

fn matching_rules<'a>(css: &'a CSS, element: &'a MapElement) -> impl Iterator<Item = &'a Styleset> {
	puffin::profile_function!();
	return css.0.iter().filter(|styleset| matches_rule(styleset, element));
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
		Selector::ChildRelation(..) => false,
	}
}

fn matches_subselector(subselector: &SubSelector, element: &MapElement) -> bool {
	match subselector {
		SubSelector::Object(object, _zoom, conditions, _pseudo_class) => {
			matches_object(object, element) && matches_conditions(conditions, element)
		}
		_ => false,
	}
}

fn matches_object(object: &Object, element: &MapElement) -> bool {
	match object {
		Object::Node => matches!(element.element, Element::Node { .. }),
		Object::Way => matches!(element.element, Element::Way { .. }),
		Object::Relation => matches!(element.element, Element::Relation { .. }),
		_ => false,
	}
}

fn matches_conditions(conditions: &[Condition], element: &MapElement) -> bool {
	conditions.iter().all(|condition| match condition {
		Condition::Binary(tag_key, BinaryOp::Eq, Expression::Identifier(tag_value)) => element
			.tags
			.iter()
			.any(|tag| tag_key == &tag.key && tag_value == &tag.value),
		_ => false,
	})
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

		let message = format!("{message}");
		let report = Report::build(ReportKind::Error, (&self.file_name, span.into_range()))
			// .with_message()
			.with_label(
				Label::new((&self.file_name, span.into_range()))
					.with_message(message)
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
