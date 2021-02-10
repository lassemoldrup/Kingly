use std::mem::transmute;
use std::ops::Not;

#[derive(PartialEq, Debug, Copy, Clone)]
#[repr(u8)]
pub enum Color {
    White = 0,
    Black = !0,
}

impl Not for Color {
    type Output = Self;

    fn not(self) -> Self::Output {
        unsafe {
            transmute(!(self as u8))
        }
    }
}