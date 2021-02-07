use crate::framework::piece::PieceKind;
use crate::framework::square::Square;
use arrayvec::ArrayVec;
use crate::framework::Side;
use std::slice::Iter;
//use std::fmt;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Move {
    Regular(Square, Square),
    Castling(Side),
    Promotion(Square, Square, PieceKind),
    EnPassant(Square, Square),
}

/*impl fmt::Display for move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            move::Regular(from, to) |
            move::Castling(from, to) |
            move::EnPassant(from, to) =>
                write!(f, "{}{}", from, to),
            move::Promotion(from, to, kind) =>
                write!(f, "{}{}{}", from, to, Into::<char>::into(*kind)),
        }
    }
}*/

pub struct MoveList(ArrayVec<[Move; 256]>);

impl MoveList {
    pub fn new() -> Self {
        MoveList(ArrayVec::new())
    }

    pub fn push(&mut self, m: Move) {
        unsafe {
            self.0.push_unchecked(m);
        }
    }

    pub fn contains(&self, m: Move) -> bool {
        self.0.contains(&m)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> Iter<Move> {
        self.0.iter()
    }
}