use std::ops::BitXor;

use crate::types::{Square, Color, Piece, CastlingRights, Bitboard};
use crate::tables::Tables;

#[cfg(test)]
mod tests;

pub trait ZobristKey {
    fn key(&self, tables: &Tables) -> u64;
}

/// Zobrist keys for piece at square
impl ZobristKey for (Piece, Square) {
    fn key(&self, tables: &Tables) -> u64 {
        let pce_index = match self.0.color() {
            Color::White => 0,
            Color::Black => 1,
        } * 6 + self.0.kind() as usize;
        let index = pce_index * 64 + self.1 as usize;

        tables.zobrist_randoms_pieces[index]
    }
}

/// Zobrist keys for piece at each square in bitboard
impl ZobristKey for (Piece, Bitboard) {
    fn key(&self, tables: &Tables) -> u64 {
        self.1.into_iter()
            .map(|sq| (self.0, sq).key(tables))
            .fold(0, BitXor::bitxor)
    }
}

/// Zobrist keys for side to move
impl ZobristKey for Color {
    fn key(&self, tables: &Tables) -> u64 {
        match self {
            Color::White => tables.zobrist_randoms_to_move,
            Color::Black => 0,
        }
    }
}

/// Zobrist keys for castling rights
impl ZobristKey for CastlingRights {
    fn key(&self, tables: &Tables) -> u64 {
        tables.zobrist_randoms_castling[usize::from(*self)]
    }
}

/// Zobrist keys for en passant squares
impl ZobristKey for Option<Square> {
    fn key(&self, tables: &Tables) -> u64 {
        match *self {
            Some(sq) => tables.zobrist_randoms_en_passant[sq as usize % 8],
            None => 0,
        }
    }
}