use crate::framework::square::Square;
use crate::standard::Bitboard;
use crate::bb;

use super::Tables;

#[test]
fn bishop_masks_initialized_correctly() {
    let tables = Tables::get();

    use Square::*;
    assert_eq!(tables.bishop_masks[A8], bb!(B7, C6, D5, E4, F3, G2));
    assert_eq!(tables.bishop_masks[B2], bb!(C3, D4, E5, F6, G7));
    assert_eq!(tables.bishop_masks[D5], bb!(E6, F7, E4, F3, G2, C4, B3, C6, B7));
}

#[test]
fn rook_masks_initialized_correctly() {
    let tables = Tables::get();

    use Square::*;
    assert_eq!(tables.rook_masks[A8], bb!(B8, C8, D8, E8, F8, G8, A7, A6, A5, A4, A3, A2));
    assert_eq!(tables.rook_masks[B2], bb!(B3, B4, B5, B6, B7, C2, D2, E2, F2, G2));
    assert_eq!(tables.rook_masks[D5], bb!(E5, F5, G5, D4, D3, D2, C5, B5, D6, D7));
}

#[test]
fn line_through_initialized_correctly() {
    let tables = Tables::get();

    use Square::*;
    assert_eq!(tables.line_through[B1][B5], bb!(B1, B2, B3, B4, B5, B6, B7, B8));
    assert_eq!(tables.line_through[F8][C5], bb!(A3, B4, C5, D6, E7, F8));
    assert_eq!(tables.line_through[D4][E4], bb!(A4, B4, C4, D4, E4, F4, G4, H4));
    assert_eq!(tables.line_through[A8][H1], bb!(A8, B7, C6, D5, E4, F3, G2, H1));
    assert_eq!(tables.line_through[C4][D6], Bitboard::new());
}

#[test]
fn ray_to_initialized_correctly() {
    let tables = Tables::get();

    assert_eq!(tables.ray_to[Square::B1][Square::B5], bb!(Square::B2, Square::B3, Square::B4, Square::B5));
    assert_eq!(tables.ray_to[Square::F8][Square::C5], bb!(Square::E7, Square::D6, Square::C5));
    assert_eq!(tables.ray_to[Square::C4][Square::D6], Bitboard::new());
}