use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use std::slice::Iter;

use arrayvec::{ArrayVec, IntoIter};

use crate::framework::piece::PieceKind;
use crate::framework::square::Square;
use std::ops::{Index, Deref};


#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Move {
    Regular(Square, Square),
    Castling(Square, Square),
    Promotion(Square, Square, PieceKind),
    EnPassant(Square, Square),
}

impl Move {
    pub fn from(self) -> Square {
        match self {
            Move::Regular(from, _) |
            Move::Castling(from, _) |
            Move::Promotion(from, _, _) |
            Move::EnPassant(from, _) => from
        }
    }

    pub fn to(self) -> Square {
        match self {
            Move::Regular(_, to) |
            Move::Castling(_, to) |
            Move::Promotion(_, to, _) |
            Move::EnPassant(_, to) => to
        }
    }

    pub fn try_from(value: &str, legal_moves: &[Move]) -> Result<Self, String> {
        if value.len() < 4 || value.len() > 5 {
            return Err(format!("Invalid move '{}'", value));
        }

        let from = Square::try_from(&value[..2])?;
        let to = Square::try_from(&value[2..4])?;

        legal_moves.iter()
            .find(|mv| mv.from() == from && mv.to() == to)
            .ok_or(format!("Illegal move '{}'", value))
            .map(|mv| *mv)
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Move::Regular(from, to) |
            Move::Castling(from, to) |
            Move::EnPassant(from, to) => write!(f, "{}{}", from, to),
            Move::Promotion(from, to, kind) => write!(f, "{}{}{}", from, to, kind),
        }
    }
}


#[derive(Debug)]
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

    pub fn get(&self, index: usize) -> Option<&Move> {
        self.0.get(index)
    }
}

impl IntoIterator for MoveList {
    type Item = Move;
    type IntoIter = IntoIter<[Move; 256]>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a MoveList {
    type Item = &'a Move;
    type IntoIter = Iter<'a, Move>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Index<usize> for MoveList {
    type Output = Move;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl AsRef<[Move]> for MoveList {
    fn as_ref(&self) -> &[Move] {
        self.0.as_slice()
    }
}

impl Deref for MoveList {
    type Target = [Move];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}