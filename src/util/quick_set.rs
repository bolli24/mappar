use either::Either::{self, Left, Right};
use std::collections::HashSet;

pub struct QuickSet<T>(Either<T, HashSet<T>>);

impl<T: Eq + core::hash::Hash> QuickSet<T> {
	pub fn new() -> Self {
		Self(Right(HashSet::new()))
	}

	pub fn from_single(element: T) -> Self {
		Self(Left(element))
	}

	pub fn insert(&mut self, new_element: T) {
		match &mut self.0 {
			Right(set) if set.is_empty() => self.0 = Left(new_element),
			Right(set) => {
				set.insert(new_element);
			}
			Left(existing) if *existing != new_element => {
				let existing = std::mem::replace(&mut self.0, Right(HashSet::new()));
				let Left(existing) = existing else { unreachable!() }; // We have to do this becaus unwrap_left/expect_left requires right to be Debug and vice versa
				let Right(set) = &mut self.0 else { unreachable!() };
				*set = HashSet::from([existing, new_element]);
			}
			_ => {}
		}
	}

	pub fn contains(&self, id: &T) -> bool {
		match &self.0 {
			Left(existing) => existing == id,
			Right(set) => set.contains(id),
		}
	}

	pub fn iter(&self) -> Iter<T> {
		match &self.0 {
			Left(element) => Iter(Left(Some(element))),
			Right(set) => Iter(Right(set.iter())),
		}
	}

	/// Get the amount of elements in the set. We ensure that this can only be 1 when Left(_)
	pub fn len(&self) -> usize {
		match &self.0 {
			Left(_) => 1,
			Right(set) => set.len(),
		}
	}

	pub fn remove(&mut self, value: &T) -> bool {
		match &mut self.0 {
			Left(element) if element == value => {
				self.0 = Right(HashSet::new());
				true
			}
			Right(set) => {
				set.remove(value);
				if set.len() == 1 {
					let element = set.drain().next().expect("Set must contain one element");
					self.0 = Left(element)
				}
				true
			}
			_ => false,
		}
	}

	pub fn single(&self) -> Option<&T> {
		self.0.as_ref().left()
	}

	pub fn is_empty(&self) -> bool {
		match &self.0 {
			Left(_) => false,
			Right(set) => set.is_empty(),
		}
	}
}

impl<T: Eq + core::hash::Hash> From<QuickSet<T>> for HashSet<T> {
	fn from(val: QuickSet<T>) -> Self {
		match val.0 {
			Left(element) => HashSet::from([element]),
			Right(set) => set,
		}
	}
}

impl<T: Eq + core::hash::Hash + Clone> Clone for QuickSet<T> {
	fn clone(&self) -> Self {
		match &self.0 {
			Left(element) => Self(Left(element.clone())),
			Right(set) => Self(Right(set.clone())),
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
			Some(value) => Self(Left(value)),
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

pub struct Iter<'a, T>(Either<Option<&'a T>, std::collections::hash_set::Iter<'a, T>>);

impl<'a, T> Iterator for Iter<'a, T> {
	type Item = &'a T;

	fn next(&mut self) -> Option<Self::Item> {
		match &mut self.0 {
			Left(value) => value.take(),
			Right(iter) => iter.next(),
		}
	}
}

pub struct IntoIter<T>(Either<Option<T>, std::collections::hash_set::IntoIter<T>>);

impl<T: Eq + core::hash::Hash> IntoIterator for QuickSet<T> {
	type Item = T;
	type IntoIter = IntoIter<T>;

	fn into_iter(self) -> Self::IntoIter {
		match self.0 {
			Left(element) => IntoIter(Left(Some(element))),
			Right(set) => IntoIter(Right(set.into_iter())),
		}
	}
}

impl<T> Iterator for IntoIter<T> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		match &mut self.0 {
			Left(value) => value.take(),
			Right(iter) => iter.next(),
		}
	}
}

#[cfg(test)]
mod test {
	// TODO:
}
