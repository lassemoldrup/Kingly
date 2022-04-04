use crate::types::{Color, Side};

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct CastlingRights(usize);

impl CastlingRights {
    pub fn new(w_king: bool, w_queen: bool, b_king: bool, b_queen: bool) -> Self {
        CastlingRights(w_king as usize 
            | (w_queen as usize) << 1
            | (b_king as usize) << 2
            | (b_queen as usize) << 3)
    }

    pub fn get(&self, color: Color, side: Side) -> bool {
        match color {
            Color::White => match side {
                Side::KingSide => self.0 & 1 == 1,
                Side::QueenSide => self.0 & 2 == 2,
            },
            Color::Black => match side {
                Side::KingSide => self.0 & 4 == 4,
                Side::QueenSide => self.0 & 8 == 8,
            },
        }
    }

    /// Sets king and queen castling rights for a given color based on a 2-bit number,
    /// e.g. 0b01 means giving kingside castling, 0b11 means giving both sided castling
    pub fn set(&mut self, color: Color, rights: usize) {
        match color {
            Color::White => self.0 |= rights,
            Color::Black => self.0 |= rights << 2,
        };
    }

    /// Similar to set, except it removes castling rights,
    /// e.g. 0b10 removes queenside castling
    pub fn remove(&mut self, color: Color, rights: usize) {
        match color {
            Color::White => self.0 &= !rights,
            Color::Black => self.0 &= !(rights << 2),
        };
    }
}

impl From<CastlingRights> for usize {
    fn from(castling: CastlingRights) -> Self {
        castling.0
    }
}
