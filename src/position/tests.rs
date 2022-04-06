use crate::fen::STARTING_FEN;
use crate::types::{Color, Move, Piece, PieceKind, Side, Square};

use super::Position;

#[test]
fn pieces_placed_correctly_in_starting_pos_fen() {
    let position = Position::from_fen(STARTING_FEN).unwrap();
    let pieces = &position.pieces;

    use Color::*;
    use PieceKind::*;
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

    let position = Position::from_fen(STARTING_FEN).unwrap();
    assert_eq!(position.to_move, White);

    let position =
        Position::from_fen("rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq - 0 1").unwrap();
    assert_eq!(position.to_move, Black);
}

#[test]
fn castling_rights_parsed_correctly_from_fen() {
    use Color::*;
    use Side::*;

    let position =
        Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w Kq - 0 1").unwrap();
    assert!(position.castling.get(White, KingSide));
    assert!(!position.castling.get(White, QueenSide));
    assert!(!position.castling.get(Black, KingSide));
    assert!(position.castling.get(Black, QueenSide));

    let position =
        Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w Qk - 0 1").unwrap();
    assert!(!position.castling.get(White, KingSide));
    assert!(position.castling.get(White, QueenSide));
    assert!(position.castling.get(Black, KingSide));
    assert!(!position.castling.get(Black, QueenSide));
}

#[test]
fn en_passant_parsed_correctly_from_fen() {
    let position = Position::from_fen(STARTING_FEN).unwrap();
    assert_eq!(position.en_passant_sq, None);

    let position =
        Position::from_fen("rnbqkbnr/ppp2ppp/4p3/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3")
            .unwrap();
    assert_eq!(position.en_passant_sq, Some(Square::D6));
}

#[test]
fn ply_clock_parsed_correctly_from_fen() {
    let position = Position::from_fen(STARTING_FEN).unwrap();
    assert_eq!(position.ply_clock, 0);

    let position =
        Position::from_fen("rnbqkb1r/pppppppp/8/8/3Nn3/8/PPPPPPPP/RNBQKB1R w KQkq - 4 3").unwrap();
    assert_eq!(position.ply_clock, 4);
}

#[test]
fn move_number_parsed_correctly_from_fen() {
    let position = Position::from_fen(STARTING_FEN).unwrap();
    assert_eq!(position.move_number, 1);

    let position =
        Position::from_fen("rnbqkb1r/pppppppp/8/8/3Nn3/8/PPPPPPPP/RNBQKB1R w KQkq - 4 3").unwrap();
    assert_eq!(position.move_number, 3);
}

fn position_matches_fen(position: Position, fen: &str) {
    let fen_pos = Position::from_fen(fen).unwrap();

    assert_eq!(position, fen_pos)
}

#[test]
fn regular_move_made_correctly() {
    let mut position = Position::new();

    unsafe {
        position.make_move(Move::new_regular(Square::E2, Square::E4));
        position.make_move(Move::new_regular(Square::D7, Square::D5));
        position.make_move(Move::new_regular(Square::E4, Square::D5));
        position.make_move(Move::new_regular(Square::D8, Square::D5));
    }

    position_matches_fen(
        position,
        "rnb1kbnr/ppp1pppp/8/3q4/8/8/PPPP1PPP/RNBQKBNR w KQkq - 0 3",
    );
}

#[test]
fn castling_move_made_correctly() {
    let mut position =
        Position::from_fen("rnbqk2r/pppp1ppp/5n2/4p1B1/1b1P4/2NQ4/PPP1PPPP/R3KBNR b KQkq - 5 4")
            .unwrap();

    unsafe {
        position.make_move(Move::new_castling(Square::E8, Square::G8));
        position.make_move(Move::new_castling(Square::E1, Square::C1));
    }

    position_matches_fen(
        position,
        "rnbq1rk1/pppp1ppp/5n2/4p1B1/1b1P4/2NQ4/PPP1PPPP/2KR1BNR b - - 7 5",
    );
}

#[test]
fn promotion_moves_made_correctly() {
    let mut position =
        Position::from_fen("rnbqkbnr/2ppppPP/8/8/8/8/PppPPP2/RNBQKBNR w KQkq - 0 9").unwrap();

    unsafe {
        position.make_move(Move::new_promotion(
            Square::G7,
            Square::H8,
            PieceKind::Queen,
        ));
        position.make_move(Move::new_promotion(
            Square::B2,
            Square::C1,
            PieceKind::Knight,
        ));
        position.make_move(Move::new_promotion(Square::H7, Square::G8, PieceKind::Rook));
        position.make_move(Move::new_promotion(
            Square::C2,
            Square::D1,
            PieceKind::Bishop,
        ));
    }

    position_matches_fen(
        position,
        "rnbqkbRQ/2pppp2/8/8/8/8/P2PPP2/RNnbKBNR w KQq - 0 11",
    );
}

#[test]
fn en_passant_moves_made_correctly() {
    let mut position =
        Position::from_fen("rnbqkbnr/ppp1pp1p/6p1/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3")
            .unwrap();

    unsafe {
        position.make_move(Move::new_en_passant(Square::E5, Square::D6));
    }

    position_matches_fen(
        position,
        "rnbqkbnr/ppp1pp1p/3P2p1/8/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 3",
    );
}

#[test]
fn regular_moves_unmade_correctly() {
    let mut position = Position::new();

    unsafe {
        position.make_move(Move::new_regular(Square::E2, Square::E4));
        position.make_move(Move::new_regular(Square::D7, Square::D5));
        position.make_move(Move::new_regular(Square::E4, Square::D5));
        position.make_move(Move::new_regular(Square::D8, Square::D5));
        position.unmake_move();
        position.unmake_move();
        position.unmake_move();
        position.unmake_move();
    }

    position_matches_fen(position, STARTING_FEN);
}

#[test]
fn castling_moves_unmade_correctly() {
    let mut position =
        Position::from_fen("rnbqk2r/pppp1ppp/5n2/4p1B1/1b1P4/2NQ4/PPP1PPPP/R3KBNR b KQkq - 5 4")
            .unwrap();

    unsafe {
        position.make_move(Move::new_castling(Square::E8, Square::G8));
        position.make_move(Move::new_castling(Square::E1, Square::C1));
        position.unmake_move();
        position.unmake_move();
    }

    position_matches_fen(
        position,
        "rnbqk2r/pppp1ppp/5n2/4p1B1/1b1P4/2NQ4/PPP1PPPP/R3KBNR b KQkq - 5 4",
    );
}

#[test]
fn promotion_moves_unmade_correctly() {
    let mut position =
        Position::from_fen("rnbqkbnr/2ppppPP/8/8/8/8/PppPPP2/RNBQKBNR w KQkq - 0 9").unwrap();

    unsafe {
        position.make_move(Move::new_promotion(
            Square::G7,
            Square::H8,
            PieceKind::Queen,
        ));
        position.make_move(Move::new_promotion(
            Square::B2,
            Square::C1,
            PieceKind::Knight,
        ));
        position.make_move(Move::new_promotion(Square::H7, Square::G8, PieceKind::Rook));
        position.make_move(Move::new_promotion(
            Square::C2,
            Square::D1,
            PieceKind::Bishop,
        ));
        position.unmake_move();
        position.unmake_move();
        position.unmake_move();
        position.unmake_move();
    }

    position_matches_fen(
        position,
        "rnbqkbnr/2ppppPP/8/8/8/8/PppPPP2/RNBQKBNR w KQkq - 0 9",
    );
}

#[test]
fn en_passant_moves_unmade_correctly() {
    let mut position =
        Position::from_fen("rnbqkbnr/ppp1pp1p/6p1/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3")
            .unwrap();

    unsafe {
        position.make_move(Move::new_en_passant(Square::E5, Square::D6));
        position.unmake_move();
    }

    position_matches_fen(
        position,
        "rnbqkbnr/ppp1pp1p/6p1/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3",
    );
}
