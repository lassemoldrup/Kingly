use crate::framework::color::Color;
use crate::framework::Side;
use crate::framework::square::Square;

pub fn get_king_sq(color: Color) -> Square {
    match color {
        Color::White => Square::E1,
        Color::Black => Square::E8,
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