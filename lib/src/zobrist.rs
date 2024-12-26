use std::ops::BitXor;

use crate::tables::Tables;
use crate::types::{Bitboard, CastlingRights, Color, Piece, Square};

pub trait ZobristKey {
    fn key(&self, tables: &Tables) -> u64;
}

/// Zobrist keys for piece at square
impl ZobristKey for (Piece, Square) {
    fn key(&self, tables: &Tables) -> u64 {
        let (pce, sq) = *self;
        let pce_index = match pce.color() {
            Color::White => 0,
            Color::Black => 1,
        } * 6
            + pce.kind() as usize;
        tables.zobrist_randoms.pieces[pce_index][sq as usize]
    }
}

/// Zobrist keys for piece at each square in bitboard
impl ZobristKey for (Piece, Bitboard) {
    fn key(&self, tables: &Tables) -> u64 {
        self.1
            .into_iter()
            .map(|sq| (self.0, sq).key(tables))
            .fold(0, u64::bitxor)
    }
}

/// Zobrist keys for side to move
impl ZobristKey for Color {
    fn key(&self, tables: &Tables) -> u64 {
        match self {
            Color::White => tables.zobrist_randoms.to_move,
            Color::Black => 0,
        }
    }
}

/// Zobrist keys for castling rights
impl ZobristKey for CastlingRights {
    fn key(&self, tables: &Tables) -> u64 {
        tables.zobrist_randoms.castling[usize::from(*self)]
    }
}

/// Zobrist keys for en passant squares
impl ZobristKey for Option<Square> {
    fn key(&self, tables: &Tables) -> u64 {
        match *self {
            Some(sq) => tables.zobrist_randoms.en_passant[sq as usize % 8],
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use itertools::{iproduct, Itertools};

    use super::ZobristKey;
    use crate::tables::Tables;
    use crate::types::{CastlingRights, Color, Piece, Square};

    #[test]
    fn all_zobrist_keys_different() {
        use Square::*;

        let tables = Tables::get_or_init();

        let mut z_keys = Vec::with_capacity(781);

        for pce in Piece::iter() {
            for sq in Square::iter() {
                z_keys.push((pce, sq).key(tables));
            }
        }

        // White and black have the same Zobrist key
        z_keys.push(Color::White.key(tables));

        for (wk, wq, bk, bq) in
            iproduct!([false, true], [false, true], [false, true], [false, true])
        {
            z_keys.push(CastlingRights::new(wk, wq, bk, bq).key(tables));
        }

        for sq in [A6, B6, C6, D6, E6, F6, G6, H6] {
            z_keys.push(Some(sq).key(tables));
        }

        assert!(z_keys.into_iter().duplicates().count() == 0);
    }
}
