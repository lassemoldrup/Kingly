//! Piece square tables for evaluation
//!
//! These tables are used to evaluate the position of a piece on the board.
//! The values used are by Tomasz Michniewski.

use crate::collections::SquareMap;
use crate::types::{Color, Piece, PieceKind, Square};

use super::piece_value;

#[inline]
pub fn piece_value_early(pce: Piece, sq: Square) -> i16 {
    match pce.color() {
        Color::White => PIECE_SQUARE_TABLES_EARLY_WHITE[pce.kind() as usize][sq],
        Color::Black => PIECE_SQUARE_TABLES_EARLY_BLACK[pce.kind() as usize][sq],
    }
}

#[inline]
pub fn piece_value_endgame(pce: Piece, sq: Square) -> i16 {
    match pce.color() {
        Color::White => PIECE_SQUARE_TABLES_ENDGAME_WHITE[pce.kind() as usize][sq],
        Color::Black => PIECE_SQUARE_TABLES_ENDGAME_BLACK[pce.kind() as usize][sq],
    }
}

static PIECE_SQUARE_TABLES_EARLY_WHITE: [SquareMap<i16>; 6] = [
    KNIGHT_SQUARE_TABLE,
    BISHOP_SQUARE_TABLE,
    ROOK_SQUARE_TABLE,
    QUEEN_SQUARE_TABLE,
    PAWN_SQUARE_TABLE,
    KING_SQUARE_TABLE,
];

static PIECE_SQUARE_TABLES_ENDGAME_WHITE: [SquareMap<i16>; 6] = [
    KNIGHT_SQUARE_TABLE,
    BISHOP_SQUARE_TABLE,
    ROOK_SQUARE_TABLE,
    QUEEN_SQUARE_TABLE,
    PAWN_SQUARE_TABLE,
    KING_SQUARE_TABLE_ENDGAME,
];

static PIECE_SQUARE_TABLES_EARLY_BLACK: [SquareMap<i16>; 6] = [
    flip(KNIGHT_SQUARE_TABLE),
    flip(BISHOP_SQUARE_TABLE),
    flip(ROOK_SQUARE_TABLE),
    flip(QUEEN_SQUARE_TABLE),
    flip(PAWN_SQUARE_TABLE),
    flip(KING_SQUARE_TABLE),
];

static PIECE_SQUARE_TABLES_ENDGAME_BLACK: [SquareMap<i16>; 6] = [
    flip(KNIGHT_SQUARE_TABLE),
    flip(BISHOP_SQUARE_TABLE),
    flip(ROOK_SQUARE_TABLE),
    flip(QUEEN_SQUARE_TABLE),
    flip(PAWN_SQUARE_TABLE),
    flip(KING_SQUARE_TABLE_ENDGAME),
];

#[rustfmt::skip]
const PAWN_SQUARE_TABLE: SquareMap<i16> = add([
    0,  0,  0,  0,  0,  0,  0,  0,
    5, 10, 10,-20,-20, 10, 10,  5,
    5, -5,-10,  0,  0,-10, -5,  5,
    0,  0,  0, 20, 20,  0,  0,  0,
    5,  5, 10, 25, 25, 10,  5,  5,
    10, 10, 20, 30, 30, 20, 10, 10,
    50, 50, 50, 50, 50, 50, 50, 50,
    0,  0,  0,  0,  0,  0,  0,  0,
], piece_value(PieceKind::Pawn));

#[rustfmt::skip]
const KNIGHT_SQUARE_TABLE: SquareMap<i16> = add([
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
], piece_value(PieceKind::Knight));

#[rustfmt::skip]
const BISHOP_SQUARE_TABLE: SquareMap<i16> = add([
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -20,-10,-10,-10,-10,-10,-10,-20,
], piece_value(PieceKind::Bishop));

#[rustfmt::skip]
const ROOK_SQUARE_TABLE: SquareMap<i16> = add([
    0,  0,  0,  5,  5,  0,  0,  0,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    5, 10, 10, 10, 10, 10, 10,  5,
    0,  0,  0,  0,  0,  0,  0,  0,
], piece_value(PieceKind::Rook));

#[rustfmt::skip]
const QUEEN_SQUARE_TABLE: SquareMap<i16> = add([
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  5,  5,  5,  5,  5,  0,-10,
    0,  0,  5,  5,  5,  5,  0, -5,
    -5,  0,  5,  5,  5,  5,  0, -5,
    -10,  0,  5,  5,  5,  5,  0,-10,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20,
], piece_value(PieceKind::Queen));

#[rustfmt::skip]
const KING_SQUARE_TABLE: SquareMap<i16> = add([
    20, 30, 10,  0,  0, 10, 30, 20,
    20, 20,  0,  0,  0,  0, 20, 20,
    -10,-20,-20,-20,-20,-20,-20,-10,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
], piece_value(PieceKind::King));

#[rustfmt::skip]
const KING_SQUARE_TABLE_ENDGAME: SquareMap<i16> = add([
    -50,-40,-30,-20,-20,-30,-40,-50,
    -30,-20,-10,  0,  0,-10,-20,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-30,  0,  0,  0,  0,-30,-30,
    -50,-30,-30,-30,-30,-30,-30,-50,
], piece_value(PieceKind::King));

const fn add(table: [i16; 64], value: i16) -> SquareMap<i16> {
    let mut new_table = table;
    let mut i = 0;
    while i < 64 {
        new_table[i] += value;
        i += 1;
    }
    SquareMap::new(new_table)
}

const fn flip(table: SquareMap<i16>) -> SquareMap<i16> {
    let table = table.inner_map();
    let mut new_table = [0; 64];
    let mut i = 0;
    while i < 64 {
        new_table[i] = -table[63 - i];
        i += 1;
    }
    SquareMap::new(new_table)
}
