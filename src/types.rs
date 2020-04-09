use num_enum::{TryFromPrimitive, UnsafeFromPrimitive};
use enum_map::Enum;
use std::convert::TryFrom;
use std::fmt;
use PieceType::*;
use Color::*;
use std::str::FromStr;

pub mod moves;
pub use moves::Move;
pub mod bitboards;
pub use bitboards::Bitboard;

/// Squares are enumerated in a little-endian rank-file mapping
#[derive(Debug, Copy, Clone, UnsafeFromPrimitive, TryFromPrimitive, Enum)]
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

impl Square{
    pub fn get(x: u8) -> Square {
        if x > 63 {
            panic!("Invalid square {}", x);
        };
        unsafe {
            Square::from_unchecked(x)
        }
    }
}

impl FromStr for Square {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() != 2 {
            Err("Invalid square")
        } else {
            match Square::try_from((chars[0] as u8 - 'a' as u8) + (chars[1] as u8 - '1' as u8) * 8) {
                Ok(s) => Ok(s),
                Err(_) => Err("Invalid square"),
            }
        }
    }
}

#[derive(UnsafeFromPrimitive)]
#[repr(u8)]
pub enum File {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
}

impl TryFrom<char> for File {
    type Error = &'static str;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        unsafe {
            match value {
                'a'..='h' => Ok(File::from_unchecked(value as u8 - 'a' as u8)),
                _ => Err("Invalid file")
            }
        }
    }
}

#[derive(UnsafeFromPrimitive)]
#[repr(u8)]
pub enum Rank {
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
    R8,
}

impl TryFrom<char> for Rank {
    type Error = &'static str;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        unsafe {
            match value {
                '1'..='8' => Ok(Rank::from_unchecked(value as u8 - '1' as u8)),
                _ => Err("Invalid rank")
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Side {
    KingSide,
    QueenSide,
}

#[derive(Debug, Copy, Clone, Enum)]
#[repr(u8)]
pub enum Color {
    White,
    Black,
}

impl TryFrom<char> for Color {
    type Error = &'static str;

    fn try_from(mut value: char) -> Result<Self, Self::Error> {
        if value.is_ascii_uppercase() {
            value = value.to_ascii_lowercase();
        }
        match value {
            'w' => Ok(White),
            'b' => Ok(Black),
            _ => Err("Not a valid color character"),
        }
    }
}

#[derive(Debug, Copy, Clone, UnsafeFromPrimitive, Enum)]
#[repr(u8)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl TryFrom<char> for PieceType {
    type Error = &'static str;

    fn try_from(mut value: char) -> Result<Self, Self::Error> {
        if value.is_ascii_uppercase() {
            value = value.to_ascii_lowercase();
        }
        match value {
            'p' => Ok(Pawn),
            'n' => Ok(Knight),
            'b' => Ok(Bishop),
            'r' => Ok(Rook),
            'q' => Ok(Queen),
            'k' => Ok(King),
            _ => Err("Not a valid piece character"),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Piece {
    pub kind: PieceType,
    pub color: Color,
}

impl Piece {
    pub fn new(kind: PieceType, color: Color) -> Self {
        Piece { kind, color }
    }
}

impl TryFrom<char> for Piece {
    type Error = &'static str;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        let kind;
        let color: Color;
        if value.is_ascii_uppercase() {
            color = White;
        } else {
            color = Black;
        };
        kind = PieceType::try_from(value)?;
        Ok(Piece { kind, color })
    }
}


impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", std::char::from_u32(0x265F - self.kind as u32 - 6 * self.color as u32).unwrap())
    }
}