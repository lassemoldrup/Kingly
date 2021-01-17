use crate::framework::{CastlingRights, Side};
use crate::framework::color::Color;

pub struct StandardCastlingRights {
    w_king: bool,
    w_queen: bool,
    b_king: bool,
    b_queen: bool,
}

impl CastlingRights for StandardCastlingRights {
    fn new(w_king: bool, w_queen: bool, b_king: bool, b_queen: bool) -> Self {
        StandardCastlingRights {
            w_king, w_queen, b_king, b_queen
        }
    }

    fn get(&self, col: Color, side: Side) -> bool {
        match col {
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

    fn set(&mut self, col: Color, side: Side, value: bool) {
        match col {
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