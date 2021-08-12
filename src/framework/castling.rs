use crate::framework::color::Color;
use crate::framework::Side;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct CastlingRights {
    w_king: bool,
    w_queen: bool,
    b_king: bool,
    b_queen: bool,
}

impl CastlingRights {
    pub fn new(w_king: bool, w_queen: bool, b_king: bool, b_queen: bool) -> Self {
        CastlingRights {
            w_king, w_queen, b_king, b_queen
        }
    }

    pub fn get(&self, color: Color, side: Side) -> bool {
        match color {
            Color::White => match side {
                Side::KingSide => self.w_king,
                Side::QueenSide => self.w_queen,
            },
            Color::Black => match side {
                Side::KingSide => self.b_king,
                Side::QueenSide => self.b_queen,
            },
        }
    }

    pub fn set(&mut self, color: Color, side: Side, value: bool) {
        match color {
            Color::White => match side {
                Side::KingSide => self.w_king = value,
                Side::QueenSide => self.w_queen = value,
            },
            Color::Black => match side {
                Side::KingSide => self.b_king = value,
                Side::QueenSide => self.b_queen = value,
            },
        };
    }
}