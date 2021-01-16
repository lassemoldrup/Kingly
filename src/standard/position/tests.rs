use crate::standard::position::StandardPosition;
use crate::framework::{Position, PieceMap};
use crate::standard::piece_map::SquareSetPieceMap;
use crate::standard::bitboard::Bitboard;
use crate::framework::square::Square;
use crate::framework::piece::{Piece, PieceKind};
use crate::framework::color::Color;
use crate::framework::fen::STARTING_FEN;

#[test]
fn pieces_placed_correctly_in_starting_pos_fen() {
    let position: StandardPosition<SquareSetPieceMap<Bitboard>> = StandardPosition::from_fen(STARTING_FEN).unwrap();
    let pieces = position.get_piece_map();

    use PieceKind::*;
    use Color::*;
    assert_eq!(pieces.get(Square::A1), Some(Piece(Rook, White)));
    assert_eq!(pieces.get(Square::E7), Some(Piece(Pawn, Black)));
    assert_eq!(pieces.get(Square::E8), Some(Piece(King, Black)));
    assert_eq!(pieces.get(Square::D1), Some(Piece(Queen, White)));
    assert_eq!(pieces.get(Square::B8), Some(Piece(Knight, Black)));
    assert_eq!(pieces.get(Square::C1), Some(Piece(Bishop, White)));
}