use crate::map::element::{Element, MapElement};
#[cfg(target_arch = "wasm32")]
use crate::spawn_task;
use crate::ui::map_widget::css::cache::{DependencyMask, StyleCache};
use crate::ui::map_widget::style::Style as MapStyle;
use crate::ui::messages::Messages;
use context::StyleContext;
use egui::mutex::RwLock;
use egui::{Color32, TextureOptions};
use epaint::{TextureId, TextureManager};
use error::{Error, Reporting};
use eval::{EvalContext, ExpressionEval};
use icon::Icon;
use log::{error, info};
use mappar_css::color::get_named_color;
use mappar_css::*;
#[cfg(target_arch = "wasm32")]
use reqwest::Client;
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

pub mod cache;
pub mod context;
mod error;
pub mod eval;
pub mod icon;

pub struct AnalyzedStyleset(Styleset, DependencyMask);

// TODO: maybe use sparse set instead of hashmap
pub struct Styles {
	rule_set: Vec<AnalyzedStyleset>,
	class_assignments: HashMap<Selectors, Vec<String>>,
	icons: Arc<RwLock<HashMap<String, Icon>>>,
	texture_manager: Arc<RwLock<TextureManager>>,
	fallback_texture: TextureId,
	reporting: RefCell<Reporting>,
	cache: StyleCache,
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

const PATH: &str = "./assets/styles/default.mapcss";
#[cfg(target_arch = "wasm32")]
const WASM_PATH: &str = "/assets/styles/default.mapcss";

pub fn get_default_styles(
	texture_manager: Arc<RwLock<TextureManager>>,
	messages: &mut Messages,
	#[cfg(target_arch = "wasm32")] on_loaded: impl FnOnce(Option<Styles>) + Send + 'static,
) -> Option<Styles> {
	info!("Getting default MapCSS styles.");
	#[cfg(not(target_arch = "wasm32"))]
	{
		let css_string = std::fs::read_to_string(PATH)
			.map_err(|err| {
				crate::message_error!(messages, "Couldn't load default styles ({PATH}): {err:?}");
			})
			.ok()?;
		parse_styles(css_string, texture_manager, messages)
	}
	#[cfg(target_arch = "wasm32")]
	{
		let messages = messages.clone();
		spawn_task!(fetch_styles_from_server(texture_manager, messages, on_loaded));
		None
	}
}

#[cfg(target_arch = "wasm32")]
async fn fetch_styles_from_server(
	texture_manager: Arc<RwLock<TextureManager>>,
	mut messages: Messages,
	on_loaded: impl FnOnce(Option<Styles>) + Send + 'static,
) {
	let origin = crate::util::get_current_url_origin();
	let url = format!("{}{}", origin, WASM_PATH);

	info!("Fetching MapCSS styles from: {url}");

	let client = Client::new();
	match client.get(&url).send().await {
		Ok(response) => match response.error_for_status() {
			Ok(response) => match response.text().await {
				Ok(css_string) => {
					let styles = parse_styles(css_string, texture_manager, &mut messages);
					on_loaded(styles);
				}
				Err(err) => {
					crate::message_error!(messages, "Failed to read CSS response: {err}");
					on_loaded(None);
				}
			},
			Err(err) => {
				crate::message_error!(messages, "Failed to fetch CSS (status error): {err}");
				on_loaded(None);
			}
		},
		Err(err) => {
			crate::message_error!(messages, "Failed to fetch CSS: {err}");
			on_loaded(None);
		}
	}
}

fn parse_styles(
	css_string: String,
	texture_manager: Arc<RwLock<TextureManager>>,
	messages: &mut Messages,
) -> Option<Styles> {
	let start = web_time::Instant::now();

	let fallback_texture = init_fallback_texture(&texture_manager);

	let rule_set = parse_str(&css_string, PATH)?
		.0
		.into_iter()
		.map(|styleset| {
			let deps = styleset
				.selectors
				.0
				.iter()
				.fold(DependencyMask::empty(), |acc, intersection| {
					intersection.0.iter().fold(acc, |mut acc2, selector| {
						if let Selector::SubSelector(sub) = selector {
							if let SubSelector::Object(_, zoom, conditions, _) = sub {
								if zoom.is_some() {
									acc2 |= DependencyMask::ZOOM;
								}
								if !conditions.is_empty() {
									acc2 |= DependencyMask::SELF_TAGS;
								}
							}
						}
						acc2
					})
				});
			AnalyzedStyleset(styleset, deps)
		})
		.collect();

	let mut styles = Styles {
		rule_set,
		class_assignments: HashMap::new(),
		icons: Arc::new(RwLock::new(HashMap::new())),
		texture_manager,
		fallback_texture,
		reporting: RefCell::new(Reporting::new(PATH.to_string(), css_string)),
		cache: StyleCache::new(),
	};

	for AnalyzedStyleset(styleset, _) in &styles.rule_set {
		let mut class_names = Vec::new();
		for rule in &styleset.declarations {
			match rule {
				Declaration::ClassAssign(class_name) => {
					class_names.push(class_name.clone());
				}
				Declaration::Style(style_rule) => {
					if style_rule.0.key == "icon-image" {
						if let Specifier::Expression(Spanned(Expression::String(file_name), _)) =
							&style_rule.0.specifier.0
						{
							styles.load_icon(file_name);
						}
					}
				}
			}
		}
		if !class_names.is_empty() {
			styles.class_assignments.insert(styleset.selectors.clone(), class_names);
		}
	}

	crate::message_info!(messages, "Loaded styles. Took {:?}.", start.elapsed());

	Some(styles)
}

impl Styles {
	pub fn get_style(&mut self, element: &MapElement, context: &mut StyleContext) -> MapStyle {
		use crate::ui::map_widget::css::cache::CachedStyle;

		let cache_key = (element.id, context.zoom_bucket);
		let cached = self.cache.0.get(&cache_key);

		let needs_update = match cached {
			None => true,
			Some(cached) => cached.generation != element.generation,
		};

		if needs_update {
			context.updated_styles += 1;
			let style = self.create_style(element, context);
			self.cache.0.insert(
				cache_key,
				CachedStyle {
					style: style.clone(),
					generation: element.generation,
				},
			);
			style
		} else {
			cached.expect("checked Some above").style.clone()
		}
	}

	fn create_style(&self, element: &MapElement, context: &StyleContext) -> MapStyle {
		let mut style = MapStyle::default();

		let stylesets = matching_rules(&self.rule_set, element, context);
		// TODO: sort styles by specificity
		for AnalyzedStyleset(
			Styleset {
				declarations: rules, ..
			},
			_deps,
		) in stylesets
		{
			self.apply_rules(element, &mut style, rules, context);
		}

		style
	}

	#[inline]
	fn apply_rules(&self, element: &MapElement, style: &mut MapStyle, rules: &[Declaration], context: &StyleContext) {
		for rule in rules {
			match rule {
				Declaration::Style(style_rule) => {
					self.apply_style_rule(element, style, style_rule, context);
				}
				Declaration::ClassAssign(_) => {}
			}
		}
	}

	#[inline]
	fn apply_style_rule(
		&self,
		element: &MapElement,
		style: &mut MapStyle,
		style_rule: &Spanned<CssStyle>,
		_context: &StyleContext,
	) {
		// puffin::profile_function!();
		let specifier = &style_rule.0.specifier;
		let key = &style_rule.0.key;
		match key.as_str() {
			"color" | "colour" => self.try_assign_color(&mut style.color, specifier, key),
			"fill-color" | "fill-colour" => self.try_assign_color(&mut style.fill_color, specifier, key),
			"width" | "size" => self.try_assign_number(element, &mut style.size, specifier, key),
			"icon-image" => {
				if let Some(icon_file) = self.try_eval_string(element, specifier, key) {
					style.icon = self.icons.read().get(&icon_file).copied();
				}
			}
			"text" => match &style_rule.0.specifier.0 {
				Specifier::Named(name) if name == "auto" => {
					let text = element.get_auto_text();
					if !text.is_empty() {
						style.text.text = text;
					}
				}
				_ => self.try_assign_string(element, &mut style.text.text, specifier, key),
			},
			"font-size" => self.try_assign_number(element, &mut style.text.font_size, specifier, key),
			"text-color" => self.try_assign_color(&mut style.text.color, specifier, key),
			"text-position" => self.try_assign_enum(&mut style.text.position, specifier, key),
			"icon-rotation" => self.try_assing_rotation(element, &mut style.icon_rotation, specifier, key),
			name => self.error(Error::UnknownProperty(name.to_string()), style_rule.1),
		}
	}

	#[inline]
	fn error(&self, error: Error, span: SimpleSpan) {
		self.reporting.borrow_mut().report(error, span);
	}

	#[inline]
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

	#[inline]
	fn try_assing_rotation(&self, element: &MapElement, number: &mut f32, specifier: &Spanned<Specifier>, key: &str) {
		if let Specifier::Rotation(rotation) = specifier.0 {
			*number = rotation as f32;
		} else {
			self.try_eval_number(element, specifier, key);
		}
	}

	#[inline]
	fn try_assign_number(&self, element: &MapElement, number: &mut f32, specifier: &Spanned<Specifier>, key: &str) {
		if let Some(value) = self.try_eval_number(element, specifier, key) {
			*number = value as f32;
		}
	}

	#[inline]
	fn try_assign_string(&self, element: &MapElement, string: &mut String, specifier: &Spanned<Specifier>, key: &str) {
		if let Some(value) = self.try_eval_string(element, specifier, key) {
			*string = value;
		}
	}

	#[inline]
	fn try_assign_color(&self, color: &mut Color32, specifier: &Spanned<Specifier>, key: &str) {
		if let Some(value) = self.try_eval_color(specifier, key) {
			*color = value;
		}
	}

	#[inline]
	fn try_eval_number(&self, element: &MapElement, specifier: &Spanned<Specifier>, key: &str) -> Option<f64> {
		self.try_eval_as(element, specifier, key, ValueType::Number)
			.map(Value::unrwap_number)
	}

	#[inline]
	fn try_eval_string(&self, element: &MapElement, specifier: &Spanned<Specifier>, key: &str) -> Option<String> {
		self.try_eval_as(element, specifier, key, ValueType::String)
			.map(Value::unwrap_string)
	}

	fn try_eval_ident_or_string(
		&self,
		element: &MapElement,
		specifier: &Spanned<Specifier>,
		key: &str,
	) -> Option<String> {
		if let Specifier::Expression(Spanned(Expression::Identifier(string), _)) = &specifier.0 {
			Some(string.clone())
		} else {
			self.try_eval_as(element, specifier, key, ValueType::String)
				.map(Value::unwrap_string)
		}
	}

	#[inline]
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

	fn try_eval_as(
		&self,
		element: &MapElement,
		specifier: &Spanned<Specifier>,
		key: &str,
		expr_type: ValueType,
	) -> Option<Value> {
		if let Specifier::Expression(expr) = &specifier.0 {
			let ctx = EvalContext { element };
			match expr.0.eval(&ctx) {
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
					self.reporting.borrow_mut().report(err, expr.1);
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
	rule_set: &'a Vec<AnalyzedStyleset>,
	element: &'a MapElement,
	context: &StyleContext,
) -> impl Iterator<Item = &'a AnalyzedStyleset> {
	// puffin::profile_function!();
	rule_set
		.iter()
		.filter(|styleset| matches_rule(&styleset.0, element, context))
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
		Selector::ChildRelation(..)
		| Selector::OverlapRelation(..)
		| Selector::ParentRelation(..)
		| Selector::SetRelation(..) => todo!(),
	}
}

fn matches_subselector(subselector: &SubSelector, element: &MapElement, context: &StyleContext) -> bool {
	let matching = match subselector {
		SubSelector::Object(object, zoom, conditions, pseudo_class) => {
			matches_object(*object, element, context)
				&& matches_zoom(*zoom, context)
				&& matches_conditions(conditions, element, context)
				&& matches_pseudo_class(pseudo_class, element, context)
		}
		_ => todo!("Sub Selector type: {subselector:?}"),
	};
	matching && (subselector.pseudo_class() != Some("closed") || element.is_closed())
}

fn matches_pseudo_class(pseudo_class: &Option<String>, element: &MapElement, _context: &StyleContext) -> bool {
	let Some(name) = pseudo_class else {
		return true;
	};
	match name.as_str() {
		"closed" => element.is_closed(),
		name => todo!("Unknow pseudo class: {name}"), // TODO: add reporting
	}
}

fn matches_zoom(zoom: Option<Zoom>, context: &StyleContext) -> bool {
	zoom.is_none_or(|zoom| context.zoom_bucket > zoom.0 && context.zoom_bucket < zoom.1)
}

fn matches_object(object: Object, element: &MapElement, _context: &StyleContext) -> bool {
	match object {
		Object::Node => matches!(element.element, Element::Node { .. }),
		Object::Way => matches!(element.element, Element::Way { .. }),
		Object::Relation => matches!(element.element, Element::Relation { .. }),
		Object::Area => matches!(element.element, Element::Way { .. } | Element::Relation { .. }), // TODO: add warning if :closed pseudo class is not present
		Object::All => true,
		_ => todo!("Object type: {object:?}"),
	}
}

fn matches_conditions(conditions: &[Condition], element: &MapElement, _context: &StyleContext) -> bool {
	let eval_context = EvalContext { element };
	conditions.iter().all(|condition| match condition {
		Condition::Binary(Spanned(a, _), BinaryOp::Eq, Spanned(b, _)) => {
			let Some(tag_key) = a.eval(&eval_context).ok().and_then(Value::ident_or_string) else {
				return false;
			};
			let Some(tag_value) = b.eval(&eval_context).ok().and_then(Value::ident_or_string) else {
				return false;
			};
			element.get_tag_with_new(&tag_key) == Some(&tag_value)
		}
		Condition::Tag(tag) => element.get_tag_with_new(tag).is_some(),
		_ => todo!("Condition type: {condition:?}"),
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
