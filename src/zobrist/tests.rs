use itertools::{iproduct, Itertools};

use super::ZobristKey;
use crate::tables::Tables;
use crate::types::{CastlingRights, Color, Piece, Square};

#[test]
fn all_zobrist_keys_different() {
    use Square::*;

    let tables = Tables::get();

    let mut z_keys = Vec::with_capacity(781);

    for pce in Piece::iter() {
        for sq in Square::iter() {
            z_keys.push((pce, sq).key(tables));
        }
    }

    // White and black have the same Zobrist key
    z_keys.push(Color::White.key(tables));

    for (wk, wq, bk, bq) in iproduct!([false, true], [false, true], [false, true], [false, true]) {
        z_keys.push(CastlingRights::new(wk, wq, bk, bq).key(tables));
    }

    for sq in [A6, B6, C6, D6, E6, F6, G6, H6] {
        z_keys.push(Some(sq).key(tables));
    }

    assert!(z_keys.into_iter().duplicates().count() == 0);
}
