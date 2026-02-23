use mappar_css::{
	ChildRelation, Condition, Intersection, OverlapRelation, ParentRelation, Selector, SetRelation, Styleset,
	SubSelector,
};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub(crate) struct Specificity {
	id_like: u32,
	class_like: u32,
	type_like: u32,
}

impl Ord for Specificity {
	fn cmp(&self, other: &Self) -> Ordering {
		(self.id_like, self.class_like, self.type_like).cmp(&(other.id_like, other.class_like, other.type_like))
	}
}

impl PartialOrd for Specificity {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Specificity {
	const fn new(id_like: u32, class_like: u32, type_like: u32) -> Self {
		Self {
			id_like,
			class_like,
			type_like,
		}
	}

	const fn add(mut self, other: Self) -> Self {
		self.id_like += other.id_like;
		self.class_like += other.class_like;
		self.type_like += other.type_like;
		self
	}
}

pub(crate) fn matched_rule_specificity_for<'a>(
	matched_intersections: impl IntoIterator<Item = &'a Intersection>,
) -> Option<Specificity> {
	matched_intersections.into_iter().map(intersection_specificity).max()
}

pub(crate) fn rule_specificity(styleset: &Styleset) -> Option<Specificity> {
	matched_rule_specificity_for(styleset.selectors.0.iter())
}

fn intersection_specificity(intersection: &Intersection) -> Specificity {
	let mut specificity = Specificity::default();
	for selector in &intersection.0 {
		specificity = specificity.add(selector_specificity(selector));
	}
	specificity
}

fn selector_specificity(selector: &Selector) -> Specificity {
	match selector {
		Selector::SubSelector(subselector) => subselector_specificity(subselector),
		Selector::ChildRelation(ChildRelation(a, b, link)) | Selector::ParentRelation(ParentRelation(a, b, link)) => {
			let link_spec = if link.is_some() {
				Specificity::new(0, 1, 0)
			} else {
				Specificity::default()
			};
			subselector_specificity(a)
				.add(subselector_specificity(b))
				.add(link_spec)
		}
		Selector::OverlapRelation(OverlapRelation(a, b)) | Selector::SetRelation(SetRelation(a, b, _)) => {
			subselector_specificity(a).add(subselector_specificity(b))
		}
	}
}

fn subselector_specificity(subselector: &SubSelector) -> Specificity {
	match subselector {
		SubSelector::Object(object, zoom, conditions, pseudo_class) => {
			let mut class_like: u32 = conditions.iter().map(condition_specificity).sum();
			if zoom.is_some() {
				class_like += 1;
			}
			if pseudo_class.is_some() {
				class_like += 1;
			}
			let type_like = if matches!(object, mappar_css::Object::All) {
				0
			} else {
				1
			};
			Specificity::new(0, class_like, type_like)
		}
		SubSelector::Class(_)
		| SubSelector::PseudoClass(_)
		| SubSelector::NotClass(_)
		| SubSelector::NotPseudoClass(_)
		| SubSelector::Setting(_)
		| SubSelector::SettingGroup(_) => Specificity::new(0, 1, 0),
		SubSelector::Tests(conditions) => {
			let class_like: u32 = conditions.iter().map(condition_specificity).sum();
			Specificity::new(0, class_like.max(1), 0)
		}
	}
}

fn condition_specificity(condition: &Condition) -> u32 {
	match condition {
		Condition::Not(inner) => condition_specificity(inner),
		Condition::Tag(_)
		| Condition::Truthy(_)
		| Condition::Falsy(_)
		| Condition::Regex(_, _)
		| Condition::TagRegex(_)
		| Condition::Function(_)
		| Condition::Binary(_, _, _) => 1,
	}
}

#[cfg(test)]
mod tests {
	use super::rule_specificity;
	use mappar_css::parse_str;

	fn parse_specificity(css: &str) -> Vec<super::Specificity> {
		let path = "test.mapcss";
		let rules = parse_str(css, path).expect("test styles must parse");
		rules
			.0
			.iter()
			.map(|styleset| rule_specificity(styleset).expect("Rule must have at least one selector"))
			.collect()
	}

	#[test]
	fn universal_is_less_specific_than_typed_object() {
		let specificities = parse_specificity(
			r#"
			* { width: 1; }
			node { width: 2; }
		"#,
		);

		assert_eq!(specificities.len(), 2);
		assert!(specificities[0] < specificities[1]);
	}

	#[test]
	fn typed_object_still_wins_with_same_conditions() {
		let specificities = parse_specificity(
			r#"
			*[highway] { width: 1; }
			node[highway] { width: 2; }
		"#,
		);

		assert_eq!(specificities.len(), 2);
		assert!(specificities[0] < specificities[1]);
	}

	#[test]
	#[ignore = "Utility test: prints all josm.mapcss rules and specificities"]
	fn print_josm_rule_specificities() {
		let path = "css/src/test/josm.mapcss";
		let src = std::fs::read_to_string(path).expect("josm.mapcss must be readable");
		let rules = parse_str(&src, path).expect("josm.mapcss must parse");

		for (index, styleset) in rules.0.iter().enumerate() {
			let specificity = rule_specificity(styleset).expect("Rule must have at least one selector");
			println!("{index:04} {:?} {:?}", specificity, styleset.selectors);
		}
	}
}
