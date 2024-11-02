use crate::map::map_data::{Element, MapData, MapElement};
use crate::map::render::ElementId;
use crate::map::style::Style as MapStyle;
use log::{error, info, warn};
use mappar_css::color::get_named_color;
use mappar_css::*;
use std::collections::HashMap;

// TODO: maybe use sparse set instead of hashmap
pub struct Styles {
	css: CSS,
	styles: HashMap<ElementId, MapStyle>,
	class_assignments: HashMap<Selectors, Vec<String>>,
}

pub fn get_default_styles() -> Option<Styles> {
	info!("Getting default MapCSS styles.");

	const PATH: &str = "./assets/default.mapcss";
	#[cfg(not(target_arch = "wasm32"))]
	let css_string = std::fs::read_to_string(PATH)
		.map_err(|err| {
			error!("Couldn't load default styles ({PATH}): {err:?}");
		})
		.ok()?;
	#[cfg(target_arch = "wasm32")]
	let css_string = include_str!("../assets/default.mapcss");

	let css = parse_str(&css_string, PATH)?;

	let mut class_assignments: HashMap<Selectors, Vec<String>> = HashMap::new();

	for styleset in &css.0 {
		let mut class_names = Vec::new();
		for rule in &styleset.rules {
			if let Rule::ClassAssign(class_name) = rule {
				class_names.push(class_name.clone());
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
	})
}

impl Styles {
	#[cfg(not(target_arch = "wasm32"))]
	pub fn reload_from_file(&mut self) {
		info!("Reloading MapCSS styles.");
		if let Some(styles) = get_default_styles() {
			*self = styles
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

	fn create_style(&self, element: &MapElement, map_data: &MapData) -> MapStyle {
		let mut style = MapStyle::default();

		let stylesets = matching_rules(&self.css, map_data.get_element(element.id).unwrap());
		// TODO: sort styles
		for Styleset { rules, .. } in stylesets {
			apply_rules(&mut style, rules);
		}

		style
	}
}

impl Default for Styles {
	fn default() -> Self {
		Self {
			css: CSS(Vec::new()),
			styles: HashMap::new(),
			class_assignments: HashMap::new(),
		}
	}
}

fn apply_rules(style: &mut MapStyle, rules: &[Rule]) {
	for rule in rules {
		match rule {
			Rule::Style(style_rule) => {
				apply_style_rule(style, style_rule);
			}
			Rule::ClassAssign(_) => {}
		}
	}
}

fn apply_style_rule(style: &mut MapStyle, style_rule: &CssStyle) {
	match style_rule.key.as_str() {
		"color" | "colour" => match &style_rule.specifier {
			Specifier::Color(color, _name) => {
				style.color = *color;
			}
			Specifier::Named(color_name) => {
				if let Some(color) = get_named_color(color_name) {
					style.color = color
				}
			}
			specifier => warn!("Illegal specifier for color: {specifier:?}"),
		},
		"width" | "size" => match &style_rule.specifier {
			Specifier::Expression(Expression::Number(number)) => style.size = *number as f32,
			specifier => warn!("Illegal specifier for {}: {:?}", style_rule.key, specifier),
		},
		_ => {}
	}
}

fn matching_rules<'a>(css: &'a CSS, element: &'a MapElement) -> impl Iterator<Item = &'a Styleset> {
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
