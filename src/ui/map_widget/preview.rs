use egui::Rect;
use glam::DVec2;
use smallvec::{SmallVec, smallvec};

use crate::map::element::{ElementId, ElementType};
use crate::{
	map::map_data::MapData,
	ui::map_widget::{
		render::{DetailedElement, RenderElement},
		style::Style,
	},
};

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Preview {
	#[default]
	None,
	NewNodeToPath(ElementId, DVec2),
	Node(DVec2),
}

impl Preview {
	pub fn render_elements(&self, map_data: &MapData) -> SmallVec<[DetailedElement; 2]> {
		match self {
			Self::None => SmallVec::new(),
			Self::NewNodeToPath(existing_node, pos_b) => {
				let pos_a = map_data
					.get_node_pos(*existing_node)
					.expect("Node must exist.")
					.to_web_mercator();

				smallvec![
					Self::two_path(pos_a, *pos_b).hovered_and_selected(),
					Self::node(*pos_b).hovered_and_selected()
				]
			}
			Self::Node(pos) => {
				smallvec![Self::node(*pos).hovered_and_selected()]
			}
		}
	}

	fn node(pos: DVec2) -> DetailedElement {
		DetailedElement {
			id: ElementId(i64::MAX, ElementType::Node),
			pointer_distance: None,
			pointer_threshold: 0.0,
			element: RenderElement::Node(pos),
			style: Style::default(),
			bounding_box: Rect::ZERO,
		}
	}

	fn two_path(pos_a: DVec2, pos_b: DVec2) -> DetailedElement {
		DetailedElement {
			id: ElementId(i64::MAX, ElementType::Way),
			pointer_distance: None,
			pointer_threshold: 0.0,
			element: RenderElement::Path(vec![pos_a, pos_b], false),
			style: Style::default(),
			bounding_box: Rect::ZERO,
		}
	}
}
