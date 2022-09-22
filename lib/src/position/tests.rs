use crate::fen::STARTING_FEN;
use crate::mv;
use crate::types::{Color, Piece, PieceKind, Side, Square};

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

    let fen = "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq - 0 1";
    let position = Position::from_fen(fen).unwrap();
    assert_eq!(position.to_move, Black);
}

#[test]
fn castling_rights_parsed_correctly_from_fen() {
    use Color::*;
    use Side::*;

    let fen = "rnbqkbn1/pppppppr/7p/8/8/P7/RPPPPPPP/1NBQKBNR w Kq - 0 1";
    let position = Position::from_fen(fen).unwrap();
    assert!(position.castling.get(White, KingSide));
    assert!(!position.castling.get(White, QueenSide));
    assert!(!position.castling.get(Black, KingSide));
    assert!(position.castling.get(Black, QueenSide));

    let fen = "1nbqkbnr/rppppppp/p7/8/8/7P/PPPPPPPR/RNBQKBN1 w Qk - 0 1";
    let position = Position::from_fen(fen).unwrap();
    assert!(!position.castling.get(White, KingSide));
    assert!(position.castling.get(White, QueenSide));
    assert!(position.castling.get(Black, KingSide));
    assert!(!position.castling.get(Black, QueenSide));
}

#[test]
fn en_passant_parsed_correctly_from_fen() {
    let position = Position::from_fen(STARTING_FEN).unwrap();
    assert_eq!(position.en_passant_sq, None);

    let fen = "rnbqkbnr/ppp2ppp/4p3/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3";
    let position = Position::from_fen(fen).unwrap();
    assert_eq!(position.en_passant_sq, Some(Square::D6));
}

#[test]
fn ply_clock_parsed_correctly_from_fen() {
    let position = Position::from_fen(STARTING_FEN).unwrap();
    assert_eq!(position.ply_clock, 0);

    let fen = "rnbqkb1r/pppppppp/8/8/3Nn3/8/PPPPPPPP/RNBQKB1R w KQkq - 4 3";
    let position = Position::from_fen(fen).unwrap();
    assert_eq!(position.ply_clock, 4);
}

#[test]
fn move_number_parsed_correctly_from_fen() {
    let position = Position::from_fen(STARTING_FEN).unwrap();
    assert_eq!(position.move_number, 1);

    let fen = "rnbqkb1r/pppppppp/8/8/3Nn3/8/PPPPPPPP/RNBQKB1R w KQkq - 4 3";
    let position = Position::from_fen(fen).unwrap();
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
        position.make_move(mv!(E2 -> E4));
        position.make_move(mv!(D7 -> D5));
        position.make_move(mv!(E4 x D5));
        position.make_move(mv!(D8 x D5));
    }

    let res_fen = "rnb1kbnr/ppp1pppp/8/3q4/8/8/PPPP1PPP/RNBQKBNR w KQkq - 0 3";
    position_matches_fen(position, res_fen);
}

#[test]
fn castling_move_made_correctly() {
    let fen = "rnbqk2r/pppp1ppp/5n2/4p1B1/1b1P4/2NQ4/PPP1PPPP/R3KBNR b KQkq - 5 4";
    let mut position = Position::from_fen(fen).unwrap();

    unsafe {
        position.make_move(mv!(O-O b));
        position.make_move(mv!(O-O-O w));
    }

    let res_fen = "rnbq1rk1/pppp1ppp/5n2/4p1B1/1b1P4/2NQ4/PPP1PPPP/2KR1BNR b - - 7 5";
    position_matches_fen(position, res_fen);
}

#[test]
fn promotion_moves_made_correctly() {
    let fen = "rnbqkbnr/2ppppPP/8/8/8/8/PppPPP2/RNBQKBNR w KQkq - 0 9";
    let mut position = Position::from_fen(fen).unwrap();

    unsafe {
        position.make_move(mv!(G7 x H8 q));
        position.make_move(mv!(B2 x C1 n));
        position.make_move(mv!(H7 x G8 r));
        position.make_move(mv!(C2 x D1 b));
    }

    let res_fen = "rnbqkbRQ/2pppp2/8/8/8/8/P2PPP2/RNnbKBNR w KQq - 0 11";
    position_matches_fen(position, res_fen);
}

#[test]
fn en_passant_moves_made_correctly() {
    let fen = "rnbqkbnr/ppp1pp1p/6p1/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3";
    let mut position = Position::from_fen(fen).unwrap();

    unsafe {
        position.make_move(mv!(E5 ep D6));
    }

    let res_fen = "rnbqkbnr/ppp1pp1p/3P2p1/8/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 3";
    position_matches_fen(position, res_fen);
}

#[test]
fn regular_moves_unmade_correctly() {
    let mut position = Position::new();

    unsafe {
        position.make_move(mv!(E2 -> E4));
        position.make_move(mv!(D7 -> D5));
        position.make_move(mv!(E4 x D5));
        position.make_move(mv!(D8 x D5));
        position.unmake_move();
        position.unmake_move();
        position.unmake_move();
        position.unmake_move();
    }

    position_matches_fen(position, STARTING_FEN);
}

#[test]
fn castling_moves_unmade_correctly() {
    let fen = "r3k2r/pppq1ppp/2npbn2/4p1B1/1b1P4/2NQPN2/PPP1BPPP/R3K2R b KQkq - 5 4";
    let mut position = Position::from_fen(fen).unwrap();

    unsafe {
        position.make_move(mv!(O-O w));
        position.make_move(mv!(O-O b));
        position.unmake_move();
        position.unmake_move();
        position.make_move(mv!(O-O-O w));
        position.make_move(mv!(O-O-O b));
        position.unmake_move();
        position.unmake_move();
    }

    position_matches_fen(position, fen);
}

#[test]
fn promotion_moves_unmade_correctly() {
    let fen = "rnbqkbnr/2ppppPP/8/8/8/8/PppPPP2/RNBQKBNR w KQkq - 0 9";
    let mut position = Position::from_fen(fen).unwrap();

    unsafe {
        position.make_move(mv!(G7 x H8 q));
        position.make_move(mv!(B2 x C1 n));
        position.make_move(mv!(H7 x G8 r));
        position.make_move(mv!(C2 x D1 b));
    }

    position_matches_fen(position, fen);
}

#[test]
fn en_passant_moves_unmade_correctly() {
    let fen = "rnbqkbnr/ppp1pp1p/6p1/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3";
    let mut position = Position::from_fen(fen).unwrap();

    unsafe {
        position.make_move(mv!(E5 ep D6));
        position.unmake_move();
    }

    position_matches_fen(position, fen);
}
