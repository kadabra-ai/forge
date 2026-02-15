use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

/// A typed index into an Arena<T>.
/// The phantom type parameter ensures you can't use an Idx<A> to index Arena<B>.
pub struct Idx<T> {
    raw: u32,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Clone for Idx<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Idx<T> {}

impl<T> PartialEq for Idx<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<T> Eq for Idx<T> {}

impl<T> std::hash::Hash for Idx<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<T> std::fmt::Debug for Idx<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_name = std::any::type_name::<T>();
        let short_name = type_name.rsplit("::").next().unwrap_or(type_name);
        write!(f, "Idx<{}>({})", short_name, self.raw)
    }
}

impl<T> Idx<T> {
    pub fn from_raw(raw: u32) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    pub fn raw(self) -> u32 {
        self.raw
    }
}

/// A simple index-based arena. Stores elements in a Vec, returns typed Idx<T> handles.
pub struct Arena<T> {
    data: Vec<T>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn alloc(&mut self, value: T) -> Idx<T> {
        let id = Idx::from_raw(self.data.len() as u32);
        self.data.push(value);
        id
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (Idx<T>, &T)> {
        self.data
            .iter()
            .enumerate()
            .map(|(i, v)| (Idx::from_raw(i as u32), v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Idx<T>, &mut T)> {
        self.data
            .iter_mut()
            .enumerate()
            .map(|(i, v)| (Idx::from_raw(i as u32), v))
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Index<Idx<T>> for Arena<T> {
    type Output = T;
    fn index(&self, idx: Idx<T>) -> &T {
        &self.data[idx.raw as usize]
    }
}

impl<T> IndexMut<Idx<T>> for Arena<T> {
    fn index_mut(&mut self, idx: Idx<T>) -> &mut T {
        &mut self.data[idx.raw as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_and_get() {
        let mut arena: Arena<String> = Arena::new();
        let id = arena.alloc("hello".to_string());
        assert_eq!(arena[id], "hello");
    }

    #[test]
    fn alloc_returns_sequential_ids() {
        let mut arena: Arena<u32> = Arena::new();
        let a = arena.alloc(10);
        let b = arena.alloc(20);
        assert_eq!(a.raw(), 0);
        assert_eq!(b.raw(), 1);
    }

    #[test]
    fn arena_len() {
        let mut arena: Arena<u32> = Arena::new();
        assert_eq!(arena.len(), 0);
        arena.alloc(1);
        arena.alloc(2);
        assert_eq!(arena.len(), 2);
    }

    #[test]
    fn mutable_access() {
        let mut arena: Arena<String> = Arena::new();
        let id = arena.alloc("hello".to_string());
        arena[id] = "world".to_string();
        assert_eq!(arena[id], "world");
    }

    #[test]
    fn iter_over_arena() {
        let mut arena: Arena<u32> = Arena::new();
        let a = arena.alloc(10);
        let b = arena.alloc(20);
        let items: Vec<_> = arena.iter().collect();
        assert_eq!(items, vec![(a, &10), (b, &20)]);
    }
}
