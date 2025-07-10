use crate::map::map_data::{Element, ElementId, MapElement};
use crate::ui::map_widget::style::Style as MapStyle;
use context::StyleContext;
use error::{Error, Reporting};
use icon::Icon;

use egui::mutex::RwLock;
use egui::{Color32, TextureOptions};
use epaint::{TextureId, TextureManager};
use log::{error, info};
use mappar_css::color::get_named_color;
use mappar_css::eval::ValueType;
use mappar_css::*;

use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

pub mod context;
mod error;
pub mod icon;

// TODO: maybe use sparse set instead of hashmap
pub struct Styles {
	css: CSS,
	style_mapping: HashMap<ElementId, MapStyle>,
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
	const PATH: &str = "./assets/styles/default.mapcss";

	info!("Getting default MapCSS styles.");
	#[cfg(not(target_arch = "wasm32"))]
	let css_string = std::fs::read_to_string(PATH)
		.map_err(|err| {
			error!("Couldn't load default styles ({PATH}): {err:?}");
		})
		.ok()?;
	#[cfg(target_arch = "wasm32")]
	let css_string = include_str!("../../../../assets/styles/default.mapcss").to_string();

	let fallback_texture = init_fallback_texture(&texture_manager);

	let mut styles = Styles {
		css: parse_str(&css_string, PATH)?,
		style_mapping: HashMap::new(),
		class_assignments: HashMap::new(),
		icons: Arc::new(RwLock::new(HashMap::new())),
		texture_manager,
		fallback_texture,
		reporting: RefCell::new(Reporting::new(PATH.to_string(), css_string)),
	};

	for styleset in &styles.css.0 {
		let mut class_names = Vec::new();
		for rule in &styleset.rules {
			match rule {
				Rule::ClassAssign(class_name) => {
					class_names.push(class_name.clone());
				}
				Rule::Style(style_rule) => {
					if style_rule.0.key == "icon-image" {
						if let Some(file_name) = styles.try_eval_string(&style_rule.0.specifier, &style_rule.0.key) {
							styles.load_icon(&file_name);
						}
					}
				}
			}
		}
		if !class_names.is_empty() {
			styles.class_assignments.insert(styleset.selectors.clone(), class_names);
		}
	}

	Some(styles)
}

impl Styles {
	pub fn get_style(&self, element: &MapElement, context: &StyleContext) -> MapStyle {
		/* if !self.styles.contains_key(&element.id) {
			let style = self.create_style(element, map_data);
			self.styles.insert(element.id, style);
		}

		&self.styles[&element.id] */
		self.create_style(element, context)
	}

	fn create_style(&self, element: &MapElement, context: &StyleContext) -> MapStyle {
		let mut style = MapStyle::default();

		let stylesets = matching_rules(&self.css, element, context);
		// TODO: sort styles by specificity
		for Styleset { rules, .. } in stylesets {
			self.apply_rules(element, &mut style, rules, context);
		}

		style
	}

	fn apply_rules(&self, element: &MapElement, style: &mut MapStyle, rules: &[Rule], context: &StyleContext) {
		for rule in rules {
			match rule {
				Rule::Style(style_rule) => {
					self.apply_style_rule(element, style, style_rule, context);
				}
				Rule::ClassAssign(_) => {}
			}
		}
	}

	fn apply_style_rule(
		&self,
		element: &MapElement,
		style: &mut MapStyle,
		style_rule: &Spanned<CssStyle>,
		_context: &StyleContext,
	) {
		puffin::profile_function!();
		let specifier = &style_rule.0.specifier;
		let key = &style_rule.0.key;
		match key.as_str() {
			"color" | "colour" => self.try_assign_color(&mut style.color, specifier, key),
			"width" | "size" => self.try_assign_number(&mut style.size, specifier, key),
			"icon-image" => {
				if let Some(icon_file) = self.try_eval_string(specifier, key) {
					style.icon = self.icons.read().get(&icon_file).copied();
				}
			}
			"text" => match &style_rule.0.specifier.0 {
				Specifier::Named(name) if name == "auto" => style.text.text = element.get_auto_text(),
				_ => self.try_assign_string(&mut style.text.text, specifier, key),
			},
			"font-size" => self.try_assign_number(&mut style.text.font_size, specifier, key),
			"text-color" => self.try_assign_color(&mut style.text.color, specifier, key),
			"text-position" => self.try_assign_enum(&mut style.text.position, specifier, key),
			"icon-rotation" => self.try_assing_rotation(&mut style.icon_rotation, specifier, key),
			name => self.error(Error::UnknownProperty(name.to_string()), style_rule.1),
		}
	}

	fn error(&self, error: Error, span: SimpleSpan) {
		self.reporting.borrow_mut().report(error, span);
	}

	fn try_assign_enum<'a, T: TryFrom<&'a str> + strum::VariantNames>(
		&self,
		value: &mut T,
		specifier: &'a Spanned<Specifier>,
		key: &str,
	) {
		if let Specifier::Named(identifier) = &specifier.0 {
			match T::try_from(identifier) {
				Ok(identifier) => {
					*value = identifier;
				}
				Err(_) => {
					self.error(
						Error::InvalidSpecifierVariant(identifier.clone(), key.to_string(), T::VARIANTS),
						specifier.1,
					);
				}
			}
		} else {
			self.error(
				Error::InvalidSpecifierType(key.to_string(), specifier.0.clone()),
				specifier.1,
			);
		}
	}

	fn try_assing_rotation(&self, number: &mut f32, specifier: &Spanned<Specifier>, key: &str) {
		if let Specifier::Rotation(rotation) = specifier.0 {
			*number = rotation as f32;
		} else {
			self.try_eval_number(specifier, key);
		}
	}

	fn try_assign_number(&self, number: &mut f32, specifier: &Spanned<Specifier>, key: &str) {
		if let Some(value) = self.try_eval_number(specifier, key) {
			*number = value as f32;
		}
	}

	fn try_assign_string(&self, string: &mut String, specifier: &Spanned<Specifier>, key: &str) {
		if let Some(value) = self.try_eval_string(specifier, key) {
			*string = value;
		}
	}

	fn try_assign_color(&self, color: &mut Color32, specifier: &Spanned<Specifier>, key: &str) {
		if let Some(value) = self.try_eval_color(specifier, key) {
			*color = value;
		}
	}

	fn try_eval_number(&self, specifier: &Spanned<Specifier>, key: &str) -> Option<f64> {
		self.try_eval_as(specifier, key, ValueType::Number)
			.map(Value::unrwap_number)
	}

	fn try_eval_string(&self, specifier: &Spanned<Specifier>, key: &str) -> Option<String> {
		self.try_eval_as(specifier, key, ValueType::String)
			.map(Value::unwrap_string)
	}

	fn try_eval_color(&self, specifier: &Spanned<Specifier>, key: &str) -> Option<Color32> {
		match &specifier.0 {
			Specifier::Color(color, _name) => Some(*color),
			Specifier::Named(color_name) => get_named_color(color_name),
			_ => {
				self.error(
					Error::InvalidSpecifierType(key.to_string(), specifier.0.clone()),
					specifier.1,
				);
				None
			}
		}
	}

	fn try_eval_as(&self, specifier: &Spanned<Specifier>, key: &str, expr_type: ValueType) -> Option<Value> {
		if let Specifier::Expression(expr) = &specifier.0 {
			match expr.0.eval() {
				Ok(value) => {
					if value.get_type() == expr_type {
						Some(value)
					} else {
						self.error(
							Error::InvalidSpecifierType(key.to_string(), specifier.0.clone()),
							specifier.1,
						);
						None
					}
				}
				Err(err) => {
					self.reporting.borrow_mut().report(err.0, expr.1);
					None
				}
			}
		} else {
			self.reporting.borrow_mut().report(
				format!("Specifier type not supported yet: {}", specifier.0.get_type()),
				specifier.1,
			);
			None
		}
	}

	fn load_icon(&self, file_name: &str) {
		let texture_id = icon::try_get_icon_texture(
			file_name,
			&self.texture_manager,
			#[cfg(target_arch = "wasm32")]
			&self.icons,
			#[cfg(target_arch = "wasm32")]
			self.fallback_texture,
		)
		.unwrap_or_else(|err| {
			error!("Could not load icon '{file_name}': {err}");
			self.fallback_texture
		});
		self.icons.write().insert(file_name.to_string(), Icon(texture_id));
	}
}

fn matching_rules<'a>(
	css: &'a CSS,
	element: &'a MapElement,
	context: &StyleContext,
) -> impl Iterator<Item = &'a Styleset> {
	puffin::profile_function!();
	css.0.iter().filter(|styleset| matches_rule(styleset, element, context))
}

fn matches_rule(styleset: &Styleset, element: &MapElement, context: &StyleContext) -> bool {
	let result = &styleset.selectors.0.iter().any(|intersection| {
		intersection
			.0
			.iter()
			.all(|selector| matches_selector(selector, element, context))
	});

	*result
}

fn matches_selector(selector: &Selector, element: &MapElement, context: &StyleContext) -> bool {
	match selector {
		Selector::SubSelector(selector) => matches_subselector(selector, element, context),
		Selector::ChildRelation(..) => todo!(),
	}
}

fn matches_subselector(subselector: &SubSelector, element: &MapElement, context: &StyleContext) -> bool {
	let matching = match subselector {
		SubSelector::Object(object, zoom, conditions, _pseudo_class) => {
			matches_object(*object, element, context)
				&& matches_zoom(*zoom, context)
				&& matches_conditions(conditions, element, context)
		}
		_ => todo!(),
	};
	matching && (subselector.pseudo_class() != Some("closed") || element.is_closed())
}

fn matches_zoom(zoom: Option<Zoom>, context: &StyleContext) -> bool {
	zoom.is_none_or(|zoom| context.zoom_level > f32::from(zoom.0) && context.zoom_level < f32::from(zoom.1))
}

fn matches_object(object: Object, element: &MapElement, _context: &StyleContext) -> bool {
	match object {
		Object::Node => matches!(element.element, Element::Node { .. }),
		Object::Way => matches!(element.element, Element::Way { .. }),
		Object::Relation => matches!(element.element, Element::Relation { .. }),
		Object::All => true,
		_ => todo!(),
	}
}

fn matches_conditions(conditions: &[Condition], element: &MapElement, _context: &StyleContext) -> bool {
	conditions.iter().all(|condition| match condition {
		Condition::Binary(tag_key, BinaryOp::Eq, Expression::Identifier(tag_value)) => {
			element.get_tag_with_new(tag_key) == Some(tag_value)
		}
		Condition::Tag(tag) => element.get_tag_with_new(tag).is_some(),
		_ => todo!(),
	})
}

fn init_fallback_texture(texture_manager: &Arc<RwLock<TextureManager>>) -> TextureId {
	let fallback_image = egui::ColorImage {
		size: [2, 2],
		source_size: [2.0, 2.0].into(),
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
