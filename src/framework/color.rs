use std::ops::Not;

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Color {
    White,
    Black,
}

impl Not for Color {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}