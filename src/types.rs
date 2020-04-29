use num_enum::UnsafeFromPrimitive;
use std::convert::TryFrom;
use std::fmt;
use PieceType::*;
use Color::*;
use std::str::FromStr;
use std::ops::{Not, Add};
use bin_vec::BinVec;
use std::iter::FusedIterator;

//pub mod moves;
//pub use moves::Move;
pub mod bitboard;
pub use bitboard::{Bitboard};

pub mod square_map;

/// Squares are enumerated in a little-endian rank-file mapping
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, UnsafeFromPrimitive)]
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
    pub fn get_file(self) -> File {
        unsafe { File::from_unchecked(self as u8 % 8) }
    }
    pub fn get_rank(self) -> Rank {
        unsafe { Rank::from_unchecked(self as u8 / 8) }
    }
    /// Applies a vector, returning the destination square
    /// # Safety
    /// Resulting square must be valid
    pub unsafe fn add_unchecked(self, vec: BinVec) -> Self {
        let new_sq = self as i8 + vec.0;
        debug_assert!(0 <= new_sq && new_sq < 64);
        Square::from_unchecked(new_sq as u8)
    }
    pub fn range_to(self, end: Square) -> SquareRange {
        if self > end {
            SquareRange {
                next_sq: None,
                end
            }
        } else {
            SquareRange {
                next_sq: Some(self),
                end
            }
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

impl TryFrom<i8> for Square {
    type Error = ();

    fn try_from(value: i8) -> Result<Self, Self::Error> {
        if 0 <= value && value < 64 {
            unsafe {
                Ok(Square::from_unchecked(value as u8))
            }
        } else {
            Err(())
        }
    }
}

impl TryFrom<u8> for Square {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 64 {
            unsafe {
                Ok(Square::from_unchecked(value))
            }
        } else {
            Err(())
        }
    }
}

impl Add<BinVec> for Square {
    type Output = Option<Square>;

    fn add(self, rhs: BinVec) -> Self::Output {
        let dest_file = self.get_file() as i8 + rhs.files();
        if dest_file >= 0 && dest_file <= 7 {
            Square::try_from(self as i8 + rhs.0).ok()
        } else {
            None
        }
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.get_file(), self.get_rank())
    }
}

pub struct SquareRange {
    next_sq: Option<Square>,
    end: Square,
}

impl Iterator for SquareRange {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        let res = self.next_sq;
        self.next_sq = match self.next_sq {
            Some(sq) if sq != self.end => unsafe { Some(sq.add_unchecked(BinVec(1))) },
            _ => None,
        };
        res
    }
}

impl FusedIterator for SquareRange { }

#[derive(Debug, UnsafeFromPrimitive, PartialEq)]
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

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_ascii_lowercase())
    }
}

#[derive(Debug, UnsafeFromPrimitive)]
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

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).chars().last().unwrap())
    }
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

pub mod bin_vec {
    use std::ops::{Add, Neg, Mul};

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct BinVec(pub i8);

    impl BinVec {
        pub fn ranks(self) -> i8 {
            let f = self.0 % 8;
            if self.0 > 0 && f > 4 {
                self.0 / 8 + 1
            } else if f < -4 {
                self.0 / 8 - 1
            } else {
                self.0 / 8
            }
        }
        pub fn files(self) -> i8 {
            let f = self.0 % 8;
            if self.0 > 0 && f > 4 {
                f - 8
            } else if f < -4 {
                8 + f
            } else {
                f
            }
        }
    }

    impl const Add for BinVec {
        type Output = BinVec;

        fn add(self, rhs: Self) -> Self::Output {
            BinVec(self.0 + rhs.0)
        }
    }

    impl const Neg for BinVec {
        type Output = BinVec;

        fn neg(self) -> Self::Output {
            BinVec(-self.0)
        }
    }

    impl const Mul<i8> for BinVec {
        type Output = BinVec;

        fn mul(self, rhs: i8) -> Self::Output {
            BinVec(self.0 * rhs)
        }
    }

    pub const NORTH: BinVec         = BinVec(8);
    pub const EAST: BinVec          = BinVec(1);
    pub const SOUTH: BinVec         = -NORTH;
    pub const WEST: BinVec          = -EAST;
    pub const NORTH_EAST: BinVec    = NORTH + EAST;
    pub const SOUTH_EAST: BinVec    = SOUTH + EAST;
    pub const SOUTH_WEST: BinVec    = SOUTH + WEST;
    pub const NORTH_WEST: BinVec    = NORTH + WEST;

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_ranks() {
            let vec = BinVec(6);
            assert_eq!(vec.ranks(), 1);
            let vec = BinVec(18);
            assert_eq!(vec.ranks(), 2);
            let vec = BinVec(12);
            assert_eq!(vec.ranks(), 1);
            let vec = BinVec(13);
            assert_eq!(vec.ranks(), 2);
            let vec = BinVec(-6);
            assert_eq!(vec.ranks(), -1);
            let vec = BinVec(-18);
            assert_eq!(vec.ranks(), -2);
            let vec = BinVec(-12);
            assert_eq!(vec.ranks(), -1);
            let vec = BinVec(-13);
            assert_eq!(vec.ranks(), -2);
        }

        #[test]
        fn test_files() {
            let vec = BinVec(6);
            assert_eq!(vec.files(), -2);
            let vec = BinVec(18);
            assert_eq!(vec.files(), 2);
            let vec = BinVec(12);
            assert_eq!(vec.files(), 4);
            let vec = BinVec(13);
            assert_eq!(vec.files(), -3);
            let vec = BinVec(-6);
            assert_eq!(vec.files(), 2);
            let vec = BinVec(-18);
            assert_eq!(vec.files(), -2);
            let vec = BinVec(-12);
            assert_eq!(vec.files(), -4);
            let vec = BinVec(-13);
            assert_eq!(vec.files(), 3);
        }
    }
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum Color {
    White,
    Black,
}

impl Not for Color {
    type Output = Color;

    fn not(self) -> Self::Output {
        if self == White {
            Black
        } else {
            White
        }
    }
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

#[derive(Eq, PartialEq, Debug, Copy, Clone, UnsafeFromPrimitive)]
#[repr(u8)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceType {
    pub const fn array() -> [Self; 6] {
        use PieceType::*;
        [Pawn, Knight, Bishop, Rook, Queen, King]
    }
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

impl Into<char> for PieceType {
    fn into(self) -> char {
        match self {
            Pawn => 'p',
            Knight => 'n',
            Bishop => 'b',
            Rook => 'r',
            Queen => 'q',
            King => 'k',
        }
    }
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct Piece(pub Color, pub PieceType);

impl Piece {
    pub const fn color(self) -> Color {
        self.0
    }
    pub const fn kind(self) -> PieceType {
        self.1
    }
    /*pub const fn is_slider(self) -> bool {
        self.1 == Bishop || self.1 == Rook || self.1 == Queen
    }*/
}

impl TryFrom<char> for Piece {
    type Error = &'static str;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        let kind;
        let color;
        if value.is_ascii_uppercase() {
            color = White;
        } else {
            color = Black;
        };
        kind = PieceType::try_from(value)?;
        Ok(Piece(color, kind))
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", std::char::from_u32(0x265F - self.kind() as u32 - 6 * self.color() as u32).unwrap())
    }
}

/// Represents a chess move
#[derive(Debug, Copy, Clone)]
pub enum Move {
    Regular(Square, Square),
    Castling(Square, Square),
    Promotion(Square, Square, PieceType),
    EnPassant(Square, Square),
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Move::Regular(from, to) |
            Move::Castling(from, to) |
            Move::EnPassant(from, to) =>
                write!(f, "{}{}", from, to),
            Move::Promotion(from, to, kind) =>
                write!(f, "{}{}{}", from, to, Into::<char>::into(*kind)),
        }
    }
}