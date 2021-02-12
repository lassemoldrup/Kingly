use std::mem::transmute;
use std::ops::Not;

#[derive(PartialEq, Debug, Copy, Clone)]
#[repr(i8)]
pub enum Color {
    White = 1,
    Black = -1,
}

impl Not for Color {
    type Output = Self;

    fn not(self) -> Self::Output {
        unsafe {
            transmute(-(self as i8))
        }
    }
}