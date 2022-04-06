use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::{fmt, mem};

use crate::types::{PieceKind, Square};

pub enum MoveKind {
    Regular,
    Castling,
    Promotion,
    EnPassant,
}

/// A chess move. Bit layout:
/// 0-5: from sq
/// 6-11: to sq
/// 12-13: kind (0: regular, 1: castling, 2: promotion, 3: en passant)
/// 14-15: promotion (0: knight, 1: bishop, 2: rook, 4: queen)
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Move(u16);

impl Move {
    pub fn new_regular(from: Square, to: Square) -> Self {
        let mut encoding = 0;

        encoding |= from as u16;
        encoding |= (to as u16) << 6;

        Self(encoding)
    }

    pub fn new_castling(from: Square, to: Square) -> Self {
        let mut encoding = 0;

        encoding |= from as u16;
        encoding |= (to as u16) << 6;
        encoding |= 1 << 12;

        Self(encoding)
    }

    pub fn new_promotion(from: Square, to: Square, kind: PieceKind) -> Self {
        let mut encoding = 0;

        encoding |= from as u16;
        encoding |= (to as u16) << 6;
        encoding |= 2 << 12;
        encoding |= (kind as u16) << 14;

        Self(encoding)
    }

    pub fn new_en_passant(from: Square, to: Square) -> Self {
        let mut encoding = 0;

        encoding |= from as u16;
        encoding |= (to as u16) << 6;
        encoding |= 3 << 12;

        Self(encoding)
    }

    pub fn from(&self) -> Square {
        unsafe { Square::from_unchecked((self.0 & 0b111111) as u8) }
    }

    pub fn to(&self) -> Square {
        unsafe { Square::from_unchecked(((self.0 >> 6) & 0b111111) as u8) }
    }

    pub fn kind(&self) -> MoveKind {
        unsafe { mem::transmute(((self.0 >> 12) & 0b11) as u8) }
    }

    pub fn promotion(&self) -> PieceKind {
        unsafe { mem::transmute(((self.0 >> 14) & 0b11) as u8) }
    }

    pub fn try_from(value: &str, legal_moves: &[Move]) -> Result<Self, String> {
        if value.len() < 4 || value.len() > 5 {
            return Err(format!("Invalid move '{}'", value));
        }

        let from = Square::try_from(&value[..2])?;
        let to = Square::try_from(&value[2..4])?;

        legal_moves
            .iter()
            .find(|mv| mv.from() == from && mv.to() == to)
            .ok_or(format!("Illegal move '{}'", value))
            .map(|mv| *mv)
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.kind() {
            MoveKind::Regular | MoveKind::Castling | MoveKind::EnPassant => {
                write!(f, "{}{}", self.from(), self.to())
            }
            MoveKind::Promotion => write!(f, "{}{}{}", self.from(), self.to(), self.promotion()),
        }
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
        legal_moves
            .iter()
            .copied()
            .find(|&mv| {
                mv.from() == self.from
                    && mv.to() == self.to
                    && match mv.kind() {
                        MoveKind::Promotion => Some(mv.promotion()) == self.promotion,
                        _ => true,
                    }
            })
            .ok_or_else(|| format!("Illegal move '{}'", self))
    }
}

impl From<Move> for PseudoMove {
    fn from(mv: Move) -> Self {
        Self {
            from: mv.from(),
            to: mv.to(),
            promotion: match mv.kind() {
                MoveKind::Promotion => Some(mv.promotion()),
                _ => None,
            },
        }
    }
}

impl PartialEq<Move> for PseudoMove {
    fn eq(&self, other: &Move) -> bool {
        self.from == other.from()
            && self.to == other.to()
            && match other.kind() {
                MoveKind::Promotion => self.promotion == Some(other.promotion()),
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
