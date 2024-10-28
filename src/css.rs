use crate::map::map_data::{Element, MapData, MapElement};
use crate::map::render::ElementId;
use crate::map::style::Style as MapStyle;
use log::{info, warn};
use mappar_css::color::get_named_color;
use mappar_css::{*};
use std::collections::HashMap;

// TODO: maybe use sparse set instead of hashmap
pub struct Styles {
	css: CSS,
	styles: HashMap<ElementId, MapStyle>,
	class_assignments: HashMap<Selectors, Vec<String>>,
}

pub fn get_default_styles() -> Option<Styles> {
	info!("Parsing default MapCSS styles.");
	let css = parse_str(include_str!("../assets/default.mapcss"), "default.mapcss")?;

	let mut class_assignments: HashMap<Selectors, Vec<String>> = HashMap::new();

	for styleset in &css.0 {
		let mut class_names = Vec::new();
		for rule in &styleset.rules {
			match rule {
				Rule::ClassAssign(class_name) => {
					class_names.push(class_name.clone());
				}
				_ => {}
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
	pub fn get_style(&mut self, element: &MapElement, map_data: &MapData) -> &MapStyle {
		if !self.styles.contains_key(&element.id) {
			let style = self.create_style(element, map_data);
			self.styles.insert(element.id, style);
		}

		return &self.styles[&element.id];
	}

	fn create_style(&self, element: &MapElement, map_data: &MapData) -> MapStyle {
		let mut style = MapStyle::default();

		let stylesets = matching_rules(&self.css, map_data.get_element(element.id).unwrap());

		for Styleset { rules, .. } in stylesets {
			apply_rules(&mut style, &rules);
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
			Rule::ClassAssign(_) => {
				warn!("Class assignments are not supported yet.");
			}
		}
	}
}

fn apply_style_rule(style: &mut MapStyle, style_rule: &CssStyle) {
	match style_rule.key.as_str() {
		"color" | "colour" => match &style_rule.specifier {
			Specifier::Color(color, name) => {
				style.color = *color;
				if name.is_some() {
					warn!("Named colors are not supported yet.");
				}
			}
			Specifier::Named(color_name) => match get_named_color(&color_name) {
				Some(color) => style.color = color,
				None => warn!("Unrecognized color name: {color_name}"),
			},
			specifier => warn!("Illegal specifier for color: {specifier:?}"),
		},
		"width" | "size" => match &style_rule.specifier {
			Specifier::Expression(Expression::Number(number)) => style.size = *number as f32,
			specifier => warn!("Illegal specifier for {}: {:?}", style_rule.key, specifier),
		},
		string => warn!("Unsupported attribute '{string}'."),
	}
}

fn matching_rules<'a>(css: &'a CSS, element: &'a MapElement) -> impl Iterator<Item = &'a Styleset> {
	return css.0.iter().filter(|styleset| matches_rule(styleset, element));
}

fn matches_rule(styleset: &Styleset, element: &MapElement) -> bool {
	let result = &styleset.selectors.0.iter().any(|intersection| {
		if intersection.1.is_some() {
			warn!("Layers not supported");
		}
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
		Selector::ChildRelation(..) => {
			warn!("Child relations are not supported yet.");
			false
		}
	}
}

fn matches_subselector(subselector: &SubSelector, element: &MapElement) -> bool {
	match subselector {
		SubSelector::Object(object, zoom, condition, pseudo_class) => {
			if zoom.is_some() {
				warn!("Zoom selectors are not supported yet.");
			}
			if !condition.is_empty() {
				warn!("Condition selectors are not supported yet.");
			}
			if pseudo_class.is_some() {
				warn!("Pseudo class selectors are not suported yet");
			}
			match object {
				Object::Node => matches!(element.element, Element::Node { .. }),
				Object::Way => matches!(element.element, Element::Way { .. }),
				Object::Relation => matches!(element.element, Element::Relation { .. }),
				Object::Area => todo!(),
				Object::Canvas => todo!(),
				Object::All => true,
				Object::Meta => todo!(),
			}
		}
		SubSelector::Class(_) => todo!(),
		SubSelector::PseudoClass(_) => todo!(),
		SubSelector::NotClass(_) => todo!(),
		SubSelector::NotPseudoClass(_) => todo!(),
		SubSelector::Setting(_) => todo!(),
	}
}
