use std::collections::HashSet;

/// A Type for a set of elements with efficient storage for a single element
pub enum QuickSet<T> {
	Empty,
	Single(T),
	Set(HashSet<T>),
}

impl<T: Eq + core::hash::Hash> QuickSet<T> {
	pub fn new() -> Self {
		Self::Empty
	}

	pub fn from_single(element: T) -> Self {
		Self::Single(element)
	}

	pub fn insert(&mut self, new_element: T) {
		match self {
			Self::Empty => *self = Self::Single(new_element),
			Self::Set(set) => {
				set.insert(new_element);
			}
			Self::Single(existing) if *existing != new_element => {
				let existing = std::mem::replace(self, Self::Set(HashSet::new()));
				let Self::Single(existing) = existing else {
					unreachable!()
				};
				let Self::Set(set) = self else { unreachable!() };
				*set = HashSet::from([existing, new_element]);
			}
			_ => {}
		}
	}

	pub fn contains(&self, id: &T) -> bool {
		match self {
			Self::Empty => false,
			Self::Single(existing) => existing == id,
			Self::Set(set) => set.contains(id),
		}
	}

	pub fn iter(&self) -> Iter<T> {
		match self {
			Self::Empty => Iter::Empty,
			Self::Single(element) => Iter::Single(Some(element)),
			Self::Set(set) => Iter::Set(set.iter()),
		}
	}

	/// Get the amount of elements in the set.
	pub fn len(&self) -> usize {
		match self {
			Self::Empty => 0,
			Self::Single(_) => 1,
			Self::Set(set) => set.len(),
		}
	}

	pub fn remove(&mut self, value: &T) -> bool {
		match self {
			Self::Single(element) if element == value => {
				*self = Self::Empty;
				true
			}
			Self::Set(set) => {
				if set.remove(value) {
					if set.len() == 1 {
						let single = set.drain().next().expect("Set must contain one element.");
						*self = Self::Single(single)
					}
					true
				} else {
					false
				}
			}
			_ => false,
		}
	}

	pub fn single(&self) -> Option<&T> {
		match self {
			Self::Single(single) => Some(single),
			_ => None,
		}
	}

	pub fn is_empty(&self) -> bool {
		matches!(self, Self::Empty)
	}
}

impl<T: Eq + core::hash::Hash> From<QuickSet<T>> for HashSet<T> {
	fn from(val: QuickSet<T>) -> Self {
		match val {
			QuickSet::Empty => HashSet::new(),
			QuickSet::Single(element) => HashSet::from([element]),
			QuickSet::Set(set) => set,
		}
	}
}

impl<T: Eq + core::hash::Hash + Clone> Clone for QuickSet<T> {
	fn clone(&self) -> Self {
		match self {
			Self::Empty => Self::Empty,
			Self::Single(element) => Self::Single(element.clone()),
			Self::Set(set) => Self::Set(set.clone()),
		}
	}
}

impl<T: Eq + core::hash::Hash> Default for QuickSet<T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T: Eq + core::hash::Hash> From<Option<T>> for QuickSet<T> {
	fn from(value: Option<T>) -> Self {
		match value {
			Some(value) => Self::Single(value),
			None => Self::new(),
		}
	}
}

impl<T: Eq + core::hash::Hash> FromIterator<T> for QuickSet<T> {
	fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
		let mut set = Self::new();
		for value in iter {
			set.insert(value);
		}
		set
	}
}

impl<'a, T: Eq + core::hash::Hash> IntoIterator for &'a QuickSet<T> {
	type Item = &'a T;
	type IntoIter = Iter<'a, T>;

	fn into_iter(self) -> Self::IntoIter {
		QuickSet::iter(self)
	}
}

pub enum Iter<'a, T> {
	Empty,
	Single(Option<&'a T>),
	Set(std::collections::hash_set::Iter<'a, T>),
}

impl<'a, T> Iterator for Iter<'a, T> {
	type Item = &'a T;

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::Empty => None,
			Self::Single(value) => value.take(),
			Self::Set(iter) => iter.next(),
		}
	}
}

pub enum IntoIter<T> {
	Empty,
	Single(Option<T>),
	Set(std::collections::hash_set::IntoIter<T>),
}

impl<T: Eq + core::hash::Hash> IntoIterator for QuickSet<T> {
	type Item = T;
	type IntoIter = IntoIter<T>;

	fn into_iter(self) -> Self::IntoIter {
		match self {
			Self::Empty => IntoIter::Empty,
			Self::Single(element) => IntoIter::Single(Some(element)),
			Self::Set(set) => IntoIter::Set(set.into_iter()),
		}
	}
}

impl<T> Iterator for IntoIter<T> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::Empty => None,
			Self::Single(value) => value.take(),
			Self::Set(iter) => iter.next(),
		}
	}
}

#[cfg(test)]
mod test {
	// TODO:
}
