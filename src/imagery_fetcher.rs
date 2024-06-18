use std::num::NonZeroUsize;

use lru::LruCache;
use skia_safe::Image;

use crate::imagery::Imagery;

pub struct ImageryFetcher {
	cache: LruCache<(u32, u32, u32), Image>,
	imagery: Box<dyn Imagery>,
}

impl ImageryFetcher {
	pub fn new(imagery: impl Imagery + 'static) -> Self {
		Self {
			cache: LruCache::new(NonZeroUsize::new(256).unwrap()),
			imagery: Box::new(imagery),
		}
	}
}
