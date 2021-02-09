use crate::framework::Side;
use crate::framework::color::Color;
use crate::framework::square::Square;

#[derive(PartialEq, Debug)]
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

    pub fn get_castling_sq(color: Color, side: Side) -> Square {
        match color {
            Color::White => match side {
                Side::KingSide => Square::G1,
                Side::QueenSide => Square::C1,
            },
            Color::Black => match side {
                Side::KingSide => Square::G8,
                Side::QueenSide => Square::C8,
            },
        }
    }

    pub fn get_rook_sq(color: Color, side: Side) -> Square {
        match color {
            Color::White => match side {
                Side::KingSide => Square::H1,
                Side::QueenSide => Square::A1,
            },
            Color::Black => match side {
                Side::KingSide => Square::H8,
                Side::QueenSide => Square::A8,
            },
        }
    }

    pub fn get_castling_rook_sq(color: Color, side: Side) -> Square {
        match color {
            Color::White => match side {
                Side::KingSide => Square::F1,
                Side::QueenSide => Square::D1,
            },
            Color::Black => match side {
                Side::KingSide => Square::F8,
                Side::QueenSide => Square::D8,
            },
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