use super::{PieceType, Square};

/// Represents a chess move
///
/// Bit 0 - 5: from value
/// Bit 6 - 11: to value
/// Bit 12 - 14: piece type
/// Bit 15: castling
#[derive(Debug, Copy, Clone)]
pub struct Move(u32);

impl Move {
    pub fn new_regular(from: Square, to: Square, kind: PieceType) -> Self {
        let from = from as u32;
        let to = to as u32;
        let val = from | (to << 6) | ((kind as u32) << 12);
        Move(val)
    }
    pub fn new_castling(to: Square) -> Self {
        match to {
            _ => panic!("Illegal destination square for castling move: {:?}", to),
        }
    }
    pub fn from(self) -> Square {
        unsafe { Square::from_unchecked((self.0 & 0b0011_1111) as u8) }
    }
    pub fn to(self) -> Square {
        unsafe { Square::from_unchecked(((self.0 >> 6) & 0b0011_1111) as u8) }
    }
    pub fn piece_type(self) -> PieceType {
        unsafe { PieceType::from_unchecked(((self.0 >> 12) & 0b0111) as u8) }
    }
}
