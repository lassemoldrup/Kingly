use crate::standard::position::StandardPosition;
use crate::framework::Position;
use crate::standard::bitboard::Bitboard;
use arrayvec::ArrayVec;
use crate::framework::moves::Move;
use crate::framework::square::Square;
use crate::framework::piece::PieceKind;

// Unit testing for gen_pawn_moves

#[test]
fn correct_pawn_moves_in_starting_position() {
    let position: StandardPosition<Bitboard> = StandardPosition::new();

    let mut moves = ArrayVec::new();
    position.gen_pawn_moves(&mut moves);

    assert!(moves.contains(&Move::Regular(Square::A2, Square::A4)));
    assert!(moves.contains(&Move::Regular(Square::F2, Square::F4)));
    assert!(moves.contains(&Move::Regular(Square::B2, Square::B3)));
    assert!(moves.contains(&Move::Regular(Square::H2, Square::H3)));
}

#[test]
fn correct_forward_pawn_moves_for_black() {
    let position: StandardPosition<Bitboard> = StandardPosition::from_fen("rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq - 0 1").unwrap();

    let mut moves = ArrayVec::new();
    position.gen_pawn_moves(&mut moves);

    assert!(moves.contains(&Move::Regular(Square::A7, Square::A5)));
    assert!(moves.contains(&Move::Regular(Square::F7, Square::F5)));
    assert!(moves.contains(&Move::Regular(Square::B7, Square::B6)));
    assert!(moves.contains(&Move::Regular(Square::H7, Square::H6)));
}

#[test]
fn correct_captures_for_white() {
    let position: StandardPosition<Bitboard> = StandardPosition::from_fen("rnbqkb1r/p1p1p1pp/7n/1p1p1pP1/P3P3/8/1PPP1P1P/RNBQKBNR w KQkq - 1 5").unwrap();

    let mut moves = ArrayVec::new();
    position.gen_pawn_moves(&mut moves);

    assert!(moves.contains(&Move::Regular(Square::A4, Square::B5)));
    assert!(moves.contains(&Move::Regular(Square::E4, Square::D5)));
    assert!(moves.contains(&Move::Regular(Square::E4, Square::F5)));
    assert!(moves.contains(&Move::Regular(Square::G5, Square::H6)));
}

#[test]
fn correct_captures_for_black() {
    let position: StandardPosition<Bitboard> = StandardPosition::from_fen("rnbqkbnr/p1p1p1pp/8/1p1p4/P1B1p1P1/5N2/1PPP1P1P/RNBQK2R b KQkq - 1 5").unwrap();

    let mut moves = ArrayVec::new();
    position.gen_pawn_moves(&mut moves);

    assert!(moves.contains(&Move::Regular(Square::B5, Square::A4)));
    assert!(moves.contains(&Move::Regular(Square::B5, Square::C4)));
    assert!(moves.contains(&Move::Regular(Square::D5, Square::C4)));
    assert!(moves.contains(&Move::Regular(Square::E4, Square::F3)));
}

#[test]
fn correct_en_passant() {
    let position: StandardPosition<Bitboard> = StandardPosition::from_fen("rnbqkb1r/p1p1pppp/5n2/1pPpP3/6P1/8/PP1P1P1P/RNBQKBNR w KQkq d6 0 6").unwrap();

    let mut moves = ArrayVec::new();
    position.gen_pawn_moves(&mut moves);

    assert!(moves.contains(&Move::EnPassant(Square::C5, Square::D6)));
    assert!(moves.contains(&Move::EnPassant(Square::E5, Square::D6)));
    assert!(!moves.contains(&Move::EnPassant(Square::C5, Square::B6)));
}

#[test]
fn correct_promotions() {
    let position: StandardPosition<Bitboard> = StandardPosition::from_fen("r1bqk2r/pP2bpPp/2n1p3/8/8/3P1P2/PPP4P/RNBQKBNR w KQ - 1 11").unwrap();

    let mut moves = ArrayVec::new();
    position.gen_pawn_moves(&mut moves);

    assert!(moves.contains(&Move::Promotion(Square::B7, Square::A8, PieceKind::Knight)));
    assert!(moves.contains(&Move::Promotion(Square::B7, Square::B8, PieceKind::Bishop)));
    assert!(moves.contains(&Move::Promotion(Square::B7, Square::C8, PieceKind::Rook)));
    assert!(moves.contains(&Move::Promotion(Square::B7, Square::C8, PieceKind::Queen)));
    assert!(moves.contains(&Move::Promotion(Square::G7, Square::G8, PieceKind::Queen)));
    assert!(moves.contains(&Move::Promotion(Square::G7, Square::H8, PieceKind::Queen)));
    assert!(!moves.contains(&Move::Promotion(Square::B7, Square::B8, PieceKind::King)));
    assert!(!moves.contains(&Move::Promotion(Square::B7, Square::B8, PieceKind::Pawn)));
}