use super::{PieceType, Square};



/*
/// Represents a chess move
///
/// Bit 0 - 5: from value
/// Bit 6 - 11: to value
/// Bit 15: castling
/// Bit 16 - 18: promotion
/// Bit 19: en passant
struct Move(u32);

impl Move {
    pub fn new_regular(from: Square, to: Square) -> Self {
        let from = from as u32;
        let to = to as u32;
        let val = from | (to << 6);
        Move(val)
    }
    pub fn new_castling(to: Square) -> Self {
        match to {
            _ => panic!("Illegal destination square for castling move: {:?}", to),
        }
    }
    pub fn new_promotion(from: Square, to: Square, promotion: PieceType) -> Self {
        let from = from as u32;
        let to = to as u32;
        let promotion = promotion as u32;
        let val = from | (to << 6) | (promotion << 16);
        Move(val)
    }
    pub fn from(self) -> Square {
        unsafe { Square::from_unchecked((self.0 & 0b0011_1111) as u8) }
    }
    pub fn to(self) -> Square {
        unsafe { Square::from_unchecked(((self.0 >> 6) & 0b0011_1111) as u8) }
    }
    pub fn castling(self) -> bool {
        (self.0 >> 15) & 0b0001 == 0b0001
    }
    // Pawn = no promotion
    pub fn promotion(self) -> PieceType {
        unsafe { PieceType::from_unchecked(((self.0 >> 16) & 0b0111) as u8) }
    }
}*/
