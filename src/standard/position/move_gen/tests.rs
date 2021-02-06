use crate::standard::position::StandardPosition;
use crate::framework::{Position, Side};
use crate::standard::bitboard::Bitboard;
use arrayvec::ArrayVec;
use crate::framework::moves::Move;
use crate::framework::square::Square;
use crate::framework::piece::PieceKind;
use crate::standard::position::move_gen::MoveGen;
use crate::bb;

// Unit testing for gen_pawn_moves

#[test]
fn correct_pawn_moves_in_starting_position() {
    let position = StandardPosition::new();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    move_gen.gen_pawn_moves(&position, &mut moves);

    assert!(moves.contains(&Move::Regular(Square::A2, Square::A4)));
    assert!(moves.contains(&Move::Regular(Square::F2, Square::F4)));
    assert!(moves.contains(&Move::Regular(Square::B2, Square::B3)));
    assert!(moves.contains(&Move::Regular(Square::H2, Square::H3)));
    assert_eq!(moves.len(), 16);
}

#[test]
fn correct_forward_pawn_moves_for_black() {
    let position = StandardPosition::from_fen("rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq - 0 1").unwrap();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    move_gen.gen_pawn_moves(&position, &mut moves);

    assert!(moves.contains(&Move::Regular(Square::A7, Square::A5)));
    assert!(moves.contains(&Move::Regular(Square::F7, Square::F5)));
    assert!(moves.contains(&Move::Regular(Square::B7, Square::B6)));
    assert!(moves.contains(&Move::Regular(Square::H7, Square::H6)));
    assert_eq!(moves.len(), 16);
}

#[test]
fn correct_pawn_captures_for_white() {
    let position = StandardPosition::from_fen("rnbqkb1r/p1p1p1pp/7n/1p1p1pP1/P3P3/8/1PPP1P1P/RNBQKBNR w KQkq - 1 5").unwrap();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    move_gen.gen_pawn_moves(&position, &mut moves);

    assert!(moves.contains(&Move::Regular(Square::A4, Square::B5)));
    assert!(moves.contains(&Move::Regular(Square::E4, Square::D5)));
    assert!(moves.contains(&Move::Regular(Square::E4, Square::F5)));
    assert!(moves.contains(&Move::Regular(Square::G5, Square::H6)));
}

#[test]
fn correct_pawn_captures_for_black() {
    let position = StandardPosition::from_fen("rnbqkbnr/p1p1p1pp/8/1p1p4/P1B1p1P1/5N2/1PPP1P1P/RNBQK2R b KQkq - 1 5").unwrap();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    move_gen.gen_pawn_moves(&position, &mut moves);

    assert!(moves.contains(&Move::Regular(Square::B5, Square::A4)));
    assert!(moves.contains(&Move::Regular(Square::B5, Square::C4)));
    assert!(moves.contains(&Move::Regular(Square::D5, Square::C4)));
    assert!(moves.contains(&Move::Regular(Square::E4, Square::F3)));
}

#[test]
fn correct_en_passant() {
    let position = StandardPosition::from_fen("rnbqkb1r/p1p1pppp/5n2/1pPpP3/6P1/8/PP1P1P1P/RNBQKBNR w KQkq d6 0 6").unwrap();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    move_gen.gen_pawn_moves(&position, &mut moves);

    assert!(moves.contains(&Move::EnPassant(Square::C5, Square::D6)));
    assert!(moves.contains(&Move::EnPassant(Square::E5, Square::D6)));
    assert!(!moves.contains(&Move::EnPassant(Square::C5, Square::B6)));
}

#[test]
fn correct_promotions() {
    let position = StandardPosition::from_fen("r1bqk2r/pP2bpPp/2n1p3/8/8/3P1P2/PPP4P/RNBQKBNR w KQ - 1 11").unwrap();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    move_gen.gen_pawn_moves(&position, &mut moves);

    assert!(moves.contains(&Move::Promotion(Square::B7, Square::A8, PieceKind::Knight)));
    assert!(moves.contains(&Move::Promotion(Square::B7, Square::B8, PieceKind::Bishop)));
    assert!(moves.contains(&Move::Promotion(Square::B7, Square::C8, PieceKind::Rook)));
    assert!(moves.contains(&Move::Promotion(Square::B7, Square::C8, PieceKind::Queen)));
    assert!(moves.contains(&Move::Promotion(Square::G7, Square::G8, PieceKind::Queen)));
    assert!(moves.contains(&Move::Promotion(Square::G7, Square::H8, PieceKind::Queen)));
    assert!(!moves.contains(&Move::Promotion(Square::B7, Square::B8, PieceKind::King)));
    assert!(!moves.contains(&Move::Promotion(Square::B7, Square::B8, PieceKind::Pawn)));
}

#[test]
fn correct_knight_moves_for_white() {
    let position = StandardPosition::from_fen("rn1qkbnr/pp2pppp/8/3pN3/2p3b1/3P1P2/PPP1P1PP/RNBQKB1R w KQkq - 0 5").unwrap();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    unsafe {
        move_gen.gen_non_pawn_moves(PieceKind::Knight, &position, &mut moves);
    }

    assert!(moves.contains(&Move::Regular(Square::B1, Square::A3)));
    assert!(moves.contains(&Move::Regular(Square::B1, Square::D2)));
    assert!(moves.contains(&Move::Regular(Square::E5, Square::C4)));
    assert!(moves.contains(&Move::Regular(Square::E5, Square::G6)));
    assert!(moves.contains(&Move::Regular(Square::E5, Square::G4)));
    assert_eq!(moves.len(), 9);
}

#[test]
fn correct_knight_moves_for_black() {
    let position = StandardPosition::from_fen("r2qkbnr/pp2pppp/2n5/P2pN3/2p3b1/3P1P2/1PP1P1PP/RNBQKB1R b KQkq - 0 6").unwrap();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    unsafe {
        move_gen.gen_non_pawn_moves(PieceKind::Knight, &position, &mut moves);
    }

    assert!(moves.contains(&Move::Regular(Square::C6, Square::B8)));
    assert!(moves.contains(&Move::Regular(Square::C6, Square::A5)));
    assert!(moves.contains(&Move::Regular(Square::C6, Square::E5)));
    assert!(moves.contains(&Move::Regular(Square::G8, Square::F6)));
    assert!(moves.contains(&Move::Regular(Square::G8, Square::H6)));
    assert_eq!(moves.len(), 7);
}

#[test]
fn correct_non_castling_king_moves_for_white() {
    let position = StandardPosition::from_fen("r1bqkbnr/ppp1ppp1/2n5/2Pp4/4P2p/3K4/PP1P1PPP/RNBQ1BNR w kq - 0 6").unwrap();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    unsafe {
        move_gen.gen_non_pawn_moves(PieceKind::King, &position, &mut moves);
    }

    assert!(moves.contains(&Move::Regular(Square::D3, Square::E3)));
    assert!(moves.contains(&Move::Regular(Square::D3, Square::E2)));
    assert!(moves.contains(&Move::Regular(Square::D3, Square::C3)));
    assert!(moves.contains(&Move::Regular(Square::D3, Square::C2)));
    assert_eq!(moves.len(), 4);
}

#[test]
fn correct_non_castling_king_moves_for_black() {
    let position = StandardPosition::from_fen("rnb3nr/pp4bp/2ppk1pP/q5P1/P3pp2/1P2N3/2PPPP2/R1BQKBNR b KQ - 0 12").unwrap();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    unsafe {
        move_gen.gen_non_pawn_moves(PieceKind::King, &position, &mut moves);
    }

    assert!(moves.contains(&Move::Regular(Square::E6, Square::E7)));
    assert!(moves.contains(&Move::Regular(Square::E6, Square::F7)));
    assert!(moves.contains(&Move::Regular(Square::E6, Square::E5)));
    assert!(moves.contains(&Move::Regular(Square::E6, Square::D7)));
    assert_eq!(moves.len(), 4);
}

#[test]
fn both_sides_castling_moves() {
    let position = StandardPosition::from_fen("rnbqkbnr/7p/ppppppp1/1B6/3PPB2/2NQ1N2/PPP2PPP/R3K2R w KQkq - 0 8").unwrap();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    unsafe {
        move_gen.gen_non_pawn_moves(PieceKind::King, &position, &mut moves);
    }

    let king_side_count = moves.iter()
        .filter(|&&m| m == Move::Castling(Side::KingSide))
        .count();
    
    let queen_side_count = moves.iter()
        .filter(|&&m| m == Move::Castling(Side::QueenSide))
        .count();

    assert_eq!(king_side_count, 1);
    assert_eq!(queen_side_count, 1);
}

#[test]
fn no_castling_through_pieces() {
    let position = StandardPosition::from_fen("rn2k2r/ppp2ppp/3q1n2/3pp3/1b1PPPb1/1PP1B1P1/P6P/RN1QKBNR b KQkq - 0 7").unwrap();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    unsafe {
        move_gen.gen_non_pawn_moves(PieceKind::King, &position, &mut moves);
    }

    let king_side_count = moves.iter()
        .filter(|&&m| m == Move::Castling(Side::KingSide))
        .count();

    assert_eq!(king_side_count, 1);
    assert!(!moves.contains(&Move::Castling(Side::QueenSide)));
}


#[test]
fn no_castling_through_attacks() {
    let position = StandardPosition::from_fen("r1bqkb1r/pppppppp/8/6B1/3PP2P/n2Q1Nn1/PPP2PP1/R3K2R w KQkq - 0 10").unwrap();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    unsafe {
        move_gen.gen_non_pawn_moves(PieceKind::King, &position, &mut moves);
    }

    let queen_side_count = moves.iter()
        .filter(|&&m| m == Move::Castling(Side::QueenSide))
        .count();

    assert!(!moves.contains(&Move::Castling(Side::KingSide)));
    assert_eq!(queen_side_count, 1);
}

#[test]
fn no_castling_when_in_check() {
    let position = StandardPosition::from_fen("r3k2r/ppp1bppp/2nq1N2/3p4/3PP3/2P5/PP2BPPP/RNBQK2R b KQkq - 0 8").unwrap();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    unsafe {
        move_gen.gen_non_pawn_moves(PieceKind::King, &position, &mut moves);
    }

    assert!(!moves.contains(&Move::Castling(Side::KingSide)));
    assert!(!moves.contains(&Move::Castling(Side::QueenSide)));
}

#[test]
fn bishop_masks_initialized_correctly() {
    let move_gen = MoveGen::new();

    use Square::*;
    assert_eq!(move_gen.bishop_masks[A8], bb!(B7, C6, D5, E4, F3, G2));
    assert_eq!(move_gen.bishop_masks[B2], bb!(C3, D4, E5, F6, G7));
    assert_eq!(move_gen.bishop_masks[D5], bb!(E6, F7, E4, F3, G2, C4, B3, C6, B7));
}

#[test]
fn rook_masks_initialized_correctly() {
    let move_gen = MoveGen::new();

    use Square::*;
    assert_eq!(move_gen.rook_masks[A8], bb!(B8, C8, D8, E8, F8, G8, A7, A6, A5, A4, A3, A2));
    assert_eq!(move_gen.rook_masks[B2], bb!(B3, B4, B5, B6, B7, C2, D2, E2, F2, G2));
    assert_eq!(move_gen.rook_masks[D5], bb!(E5, F5, G5, D4, D3, D2, C5, B5, D6, D7));
}

#[test]
fn correct_bishop_moves() {
    let position = StandardPosition::from_fen("rnbqkbnr/1ppp1pp1/p7/4p2p/1PB1P3/8/P1PP1PPP/RNBQK1NR w KQkq - 0 5").unwrap();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    unsafe {
        move_gen.gen_non_pawn_moves(PieceKind::Bishop, &position, &mut moves);
    }

    assert!(moves.contains(&Move::Regular(Square::C1, Square::B2)));
    assert!(moves.contains(&Move::Regular(Square::C4, Square::B3)));
    assert!(moves.contains(&Move::Regular(Square::C4, Square::F7)));
    assert!(moves.contains(&Move::Regular(Square::C4, Square::E2)));
    assert!(!moves.contains(&Move::Regular(Square::C1, Square::D2)));
    assert_eq!(moves.len(), 11);
}

#[test]
fn correct_rook_moves() {
    let position = StandardPosition::from_fen("1nbqkbnr/1pppppp1/8/p2r4/4PPp1/3P3P/PPP1N3/RNBQKB1R b KQk - 1 7").unwrap();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    unsafe {
        move_gen.gen_non_pawn_moves(PieceKind::Rook, &position, &mut moves);
    }

    assert!(moves.contains(&Move::Regular(Square::D5, Square::D6)));
    assert!(moves.contains(&Move::Regular(Square::D5, Square::B5)));
    assert!(moves.contains(&Move::Regular(Square::D5, Square::H5)));
    assert!(moves.contains(&Move::Regular(Square::H8, Square::H3)));
    assert!(!moves.contains(&Move::Regular(Square::D5, Square::A5)));
    assert_eq!(moves.len(), 14);
}

#[test]
fn correct_queen_moves() {
    let position = StandardPosition::from_fen("r1bqkbnr/p1pp1p1p/1pnPp1p1/8/3Q4/8/PPP1PPPP/RNB1KBNR w KQkq - 0 5").unwrap();
    let move_gen = MoveGen::new();

    let mut moves = ArrayVec::new();
    unsafe {
        move_gen.gen_non_pawn_moves(PieceKind::Queen, &position, &mut moves);
    }

    assert!(moves.contains(&Move::Regular(Square::D4, Square::D5)));
    assert!(moves.contains(&Move::Regular(Square::D4, Square::H8)));
    assert!(moves.contains(&Move::Regular(Square::D4, Square::G4)));
    assert!(moves.contains(&Move::Regular(Square::D4, Square::E3)));
    assert!(moves.contains(&Move::Regular(Square::D4, Square::D2)));
    assert!(moves.contains(&Move::Regular(Square::D4, Square::C3)));
    assert!(moves.contains(&Move::Regular(Square::D4, Square::A4)));
    assert!(moves.contains(&Move::Regular(Square::D4, Square::B6)));
    assert!(!moves.contains(&Move::Regular(Square::D4, Square::D6)));
    assert_eq!(moves.len(), 19);
}