use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use std::slice::Iter;

use arrayvec::{ArrayVec, IntoIter};

use crate::framework::piece::PieceKind;
use crate::framework::Side;
use crate::framework::square::Square;
use std::ops::Index;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Move {
    Regular(Square, Square),
    Castling(Side),
    Promotion(Square, Square, PieceKind),
    EnPassant(Square, Square),
}

impl TryFrom<&str> for Move {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "O-O" => Ok(Self::Castling(Side::KingSide)),
            "O-O-O" => Ok(Self::Castling(Side::QueenSide)),
            mv if mv.len() == 4 => {
                let from = Square::try_from(&mv[..2])?;
                let to = Square::try_from(&mv[2..])?;
                Ok(Move::Regular(from, to))
            },
            mv if mv.len() == 5 => {
                let from = Square::try_from(&mv[..2])?;
                let to = Square::try_from(&mv[2..4])?;
                let kind = PieceKind::try_from(mv[4..].chars().next().unwrap())?;
                Ok(Move::Promotion(from, to, kind))
            },
            mv if mv.len() == 6 && &mv[4..] == "ep" => {
                let from = Square::try_from(&mv[..2])?;
                let to = Square::try_from(&mv[2..4])?;
                Ok(Move::EnPassant(from, to))
            },
            _ => Err(format!("Invalid move '{}'", value)),
        }
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Move::Regular(from, to) => write!(f, "{}{}", from, to),
            Move::Castling(side) => write!(f, "{}", match side {
                Side::KingSide => "O-O",
                Side::QueenSide => "O-O-O",
            }),
            Move::Promotion(from, to, kind) => write!(f, "{}{}{}", from, to, kind),
            Move::EnPassant(from, to) => write!(f, "{}{}ep", from, to),
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

impl Index<usize> for MoveList {
    type Output = Move;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}