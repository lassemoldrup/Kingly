use crate::standard::position::StandardPosition;
use crate::framework::{Position, PieceMap, Side, CastlingRights};
use crate::standard::piece_map::SquareSetPieceMap;
use crate::standard::bitboard::Bitboard;
use crate::framework::square::Square;
use crate::framework::piece::{Piece, PieceKind};
use crate::framework::color::Color;
use crate::framework::fen::STARTING_FEN;

#[test]
fn pieces_placed_correctly_in_starting_pos_fen() {
    let position: StandardPosition<Bitboard> = StandardPosition::from_fen(STARTING_FEN).unwrap();
    let pieces = &position.pieces;

    use PieceKind::*;
    use Color::*;
    assert_eq!(pieces.get(Square::A1), Some(Piece(Rook, White)));
    assert_eq!(pieces.get(Square::E7), Some(Piece(Pawn, Black)));
    assert_eq!(pieces.get(Square::E8), Some(Piece(King, Black)));
    assert_eq!(pieces.get(Square::D1), Some(Piece(Queen, White)));
    assert_eq!(pieces.get(Square::B8), Some(Piece(Knight, Black)));
    assert_eq!(pieces.get(Square::C1), Some(Piece(Bishop, White)));
}

#[test]
fn color_parsed_correctly_from_fen() {
    use Color::*;

    let position: StandardPosition<Bitboard> = StandardPosition::from_fen(STARTING_FEN).unwrap();
    assert_eq!(position.to_move, White);

    let position: StandardPosition<Bitboard> = StandardPosition::from_fen("rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq - 0 1").unwrap();
    assert_eq!(position.to_move, Black);
}

#[test]
fn castling_rights_parsed_correctly_from_fen() {
    use Color::*;
    use Side::*;

    let position: StandardPosition<Bitboard> = StandardPosition::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w Kq - 0 1").unwrap();
    assert!(position.castling.get(White, KingSide));
    assert!(!position.castling.get(White, QueenSide));
    assert!(!position.castling.get(Black, KingSide));
    assert!(position.castling.get(Black, QueenSide));

    let position: StandardPosition<Bitboard> = StandardPosition::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w Qk - 0 1").unwrap();
    assert!(!position.castling.get(White, KingSide));
    assert!(position.castling.get(White, QueenSide));
    assert!(position.castling.get(Black, KingSide));
    assert!(!position.castling.get(Black, QueenSide));
}

#[test]
fn en_passant_parsed_correctly_from_fen() {
    let position: StandardPosition<Bitboard> = StandardPosition::from_fen(STARTING_FEN).unwrap();
    assert_eq!(position.en_passant_sq, None);

    let position: StandardPosition<Bitboard> = StandardPosition::from_fen("rnbqkbnr/ppp2ppp/4p3/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3").unwrap();
    assert_eq!(position.en_passant_sq, Some(Square::D6));
}

#[test]
fn ply_clock_parsed_correctly_from_fen() {
    let position: StandardPosition<Bitboard> = StandardPosition::from_fen(STARTING_FEN).unwrap();
    assert_eq!(position.ply_clock, 0);

    let position: StandardPosition<Bitboard> = StandardPosition::from_fen("rnbqkb1r/pppppppp/8/8/3Nn3/8/PPPPPPPP/RNBQKB1R w KQkq - 4 3").unwrap();
    assert_eq!(position.ply_clock, 4);
}

#[test]
fn move_number_parsed_correctly_from_fen() {
    let position: StandardPosition<Bitboard> = StandardPosition::from_fen(STARTING_FEN).unwrap();
    assert_eq!(position.move_number, 1);

    let position: StandardPosition<Bitboard> = StandardPosition::from_fen("rnbqkb1r/pppppppp/8/8/3Nn3/8/PPPPPPPP/RNBQKB1R w KQkq - 4 3").unwrap();
    assert_eq!(position.move_number, 3);
}