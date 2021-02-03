use crate::framework::piece::PieceKind;
use crate::framework::square::Square;
use arrayvec::ArrayVec;
//use std::fmt;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Move {
    Regular(Square, Square),
    Castling(Square, Square),
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
    pub(crate) fn new() -> Self {
        MoveList(ArrayVec::new())
    }

    fn push(&mut self, m: Move) {
        unsafe {
            self.0.push_unchecked(m);
        }
    }
}