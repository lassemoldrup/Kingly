use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use std::ops::{Deref, Index};
use std::slice::Iter;
use std::str::FromStr;

use arrayvec::{ArrayVec, IntoIter};

use crate::framework::piece::PieceKind;
use crate::framework::square::Square;

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
pub struct MoveList(ArrayVec<Move, 256>);

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

    pub fn into_vec(self) -> Vec<Move> {
        self.0.as_slice().to_vec()
    }
}

impl IntoIterator for MoveList {
    type Item = Move;
    type IntoIter = IntoIter<Move, 256>;

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


#[derive(Copy, Clone, PartialEq, Debug)]
pub struct PseudoMove {
    from: Square,
    to: Square,
    promotion: Option<PieceKind>,
}

impl PseudoMove {
    pub fn new(from: Square, to: Square, promotion: Option<PieceKind>) -> Self {
        Self {
            from,
            to,
            promotion,
        }
    }

    pub fn into_move(self, legal_moves: &[Move]) -> Result<Move, String> {
        legal_moves.iter().copied()
            .find(|&mv| mv.from() == self.from && mv.to() == self.to && match mv {
                Move::Promotion(_, _, kind) => Some(kind) == self.promotion,
                _ => true,
            })
            .ok_or_else(|| format!("Illegal move '{}'", self))
    }
}

impl From<Move> for PseudoMove {
    fn from(mv: Move) -> Self {
        Self {
            from: mv.from(),
            to: mv.to(),
            promotion: match mv {
                Move::Promotion(_, _, kind) => Some(kind),
                _ => None
            }
        }
    }
}

impl PartialEq<Move> for PseudoMove {
    fn eq(&self, other: &Move) -> bool {
        self.from == other.from()
            && self.to == other.to()
            && match other {
                Move::Promotion(_, _, kind) => self.promotion == Some(*kind),
                _ => true,
            }
    }
}

impl FromStr for PseudoMove {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 4 || s.len() == 5 {
            let from = Square::try_from(&s[0..2])?;
            let to = Square::try_from(&s[2..4])?;

            let promotion = if s.len() == 5 {
                Some(PieceKind::try_from(s.chars().nth(4).unwrap())?)
            } else {
                None
            };

            Ok(PseudoMove {
                from,
                to,
                promotion,
            })
        } else {
            Err(format!("Invalid move '{}'", s))
        }
    }
}

impl Display for PseudoMove {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.from, self.to)
    }
}
