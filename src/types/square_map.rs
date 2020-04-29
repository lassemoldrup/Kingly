use super::Square;
use std::ops::{Index, IndexMut};

#[derive(Copy, Clone)]
pub struct SquareMap<T: Copy>([T; 64]);

impl<T: Copy> SquareMap<T> {
    pub const fn new(default: T) -> Self {
        SquareMap([default; 64])
    }
    pub const fn from_array(array: [T; 64]) -> Self {
        SquareMap(array)
    }
    pub fn as_slice(&self) -> &[T] {
        self.0.as_ref()
    }
}

impl<T: Copy> Index<Square> for SquareMap<T> {
    type Output = T;

    fn index(&self, index: Square) -> &Self::Output {
        unsafe {
            &self.0.get_unchecked(index as usize)
        }
    }
}

impl<T: Copy> IndexMut<Square> for SquareMap<T> {
    fn index_mut(&mut self, index: Square) -> &mut Self::Output {
        unsafe {
            self.0.get_unchecked_mut(index as usize)
        }
    }
}