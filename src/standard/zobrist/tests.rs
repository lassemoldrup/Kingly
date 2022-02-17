use itertools::Itertools;

use crate::{framework::{color::Color, piece::{PieceKind, Piece}, square::Square, castling::CastlingRights}, standard::{zobrist::ZobristKey, tables::Tables}};

#[test]
fn all_zobrist_keys_different() {
    use Color::*;
    use PieceKind::*;
    use Square::*;

    let tables = Tables::get();
    
    let mut z_keys = Vec::with_capacity(781);

    for color in [White, Black] {
        for kind in [Pawn, Knight, Bishop, Rook, Queen, King] {
            let pce = Piece(kind, color);
            for sq in Square::iter() {
                z_keys.push((pce, sq).key(tables));
            }
        }
    }

    // White and black have the same Zobrist key
    z_keys.push(White.key(tables));

    for wk in [false, true] {
        for wq in [false, true] {
            for bk in [false, true] {
                for bq in [false, true] {
                    z_keys.push(CastlingRights::new(wk, wq, bk, bq).key(tables));
                }
            }
        }
    }

    for sq in [A6, B6, C6, D6, E6, F6, G6, H6] {
        z_keys.push(Some(sq).key(tables));
    }

    assert!(z_keys.into_iter().duplicates().count() == 0);
}