use crate::ElementId;
use crate::ui::map_widget::css::MapStyle;
use bitflags::bitflags;
use fxhash::FxHashMap;

pub struct CachedStyle {
	pub style: MapStyle,
	pub generation: u64,
}

/// Cache key: (element_id, zoom_bucket)
pub struct StyleCache(pub(crate) FxHashMap<(ElementId, u8), CachedStyle>);

impl StyleCache {
	pub fn new() -> Self {
		Self(FxHashMap::default())
	}

	pub fn invalidate_element(&mut self, element_id: ElementId) {
		// Invalidate all zoom levels for this element
		self.0.retain(|(id, _), _| *id != element_id);

		// TODO: Invalidate elements that depend on this element
		/* if let Some(dependents) = dependency_index.style_dependents.get(&element_id) {
			for dependent in dependents {
				self.remove_all_zooms(*dependent);
			}
		}*/
	}
}

bitflags! {
	#[derive(Debug, Copy, Clone, Eq, PartialEq)]
	pub struct DependencyMask: u16 {
		const SELF_TAGS    = 0b0000_0001;
		const ZOOM         = 0b0000_0010;
	//    const CLOSED       = 0b0000_0100;
	//    const PARENT_TAGS  = 0b0000_1000;
	//    const CHILD_TAGS   = 0b0001_0000;
	//    const SELECTION    = 0b0010_0000;  // For :selected pseudo-class
	//    const MODIFIED     = 0b0100_0000;  // For :modified pseudo-class
	//    const CLASS        = 0b1000_0000;  // Depends on class assignments
	//      TODO: const HOVER ??
	}
}
