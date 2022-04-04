use std::ops::Neg;

/// The direction from white's perspective
#[derive(Copy, Clone)]
#[repr(i8)]
pub enum Direction {
    North = 8,
    NorthEast = 9,
    East = 1,
    SouthEast = -7,
    South = -8,
    SouthWest = -9,
    West = -1,
    NorthWest = 7,
}

impl Neg for Direction {
    type Output = Self;

    fn neg(self) -> Self::Output {
        unsafe {
            std::mem::transmute(-(self as i8))
        }
    }
}


#[derive(Copy, Clone)]
pub struct SquareVec(pub i8, pub i8);


#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Side {
    KingSide, QueenSide
}