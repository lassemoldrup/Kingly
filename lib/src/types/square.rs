use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use std::mem;
use std::ops::Add;

use super::Direction;
use super::SquareVec;

#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum Square {
    A1, B1, C1, D1, E1, F1, G1, H1,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A8, B8, C8, D8, E8, F8, G8, H8,
}

impl Square {
    /// # Safety
    /// `value` must be a valid square index
    pub unsafe fn from_unchecked(value: u8) -> Self {
        debug_assert!(value < 64);
        mem::transmute(value)
    }

    pub fn iter() -> SquareIter {
        SquareIter::new()
    }

    pub fn rank(self) -> u8 {
        self as u8 / 8
    }

    pub fn file(self) -> u8 {
        self as u8 % 8
    }

    /// # Safety
    /// Result must be a valid `Square`
    pub unsafe fn shift(self, dir: Direction) -> Self {
        Self::from_unchecked((self as i8 + dir as i8) as u8)
    }
}

impl TryFrom<u8> for Square {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 64 {
            unsafe { Ok(Self::from_unchecked(value)) }
        } else {
            Err(format!("{} is not a valid square index", value))
        }
    }
}

impl TryFrom<&str> for Square {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let chars: Vec<char> = value.chars().collect();
        if chars.len() == 2 && matches!(chars[0], 'a'..='h') && matches!(chars[1], '1'..='8') {
            let r = chars[1] as u8 - b'1';
            let c = chars[0] as u8 - b'a';

            Square::try_from(8 * r + c)
        } else {
            Err(format!("Invalid square '{}'", value))
        }
    }
}

//TODO: Optimize?
impl Add<SquareVec> for Square {
    type Output = Option<Square>;

    fn add(self, rhs: SquareVec) -> Self::Output {
        let rank = self.rank() as i8 + rhs.0;
        let file = self.file() as i8 + rhs.1;

        if matches!(rank, 0..=7) && matches!(file, 0..=7) {
            unsafe { Some(Square::from_unchecked((8 * rank + file) as u8)) }
        } else {
            None
        }
    }
}

impl Display for Square {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", (self.file() + b'a') as char, self.rank() + 1)
    }
}

pub struct SquareIter {
    next_idx: u8,
}

impl SquareIter {
    fn new() -> Self {
        Self { next_idx: 0 }
    }
}

impl Iterator for SquareIter {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        let result = Square::try_from(self.next_idx).ok();
        self.next_idx += 1;
        result
    }
}
