use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use crate::move_gen::MoveGenState;
use crate::position::Position;
use crate::tables::Tables;
use crate::types::{PieceKind, Square};
use crate::{bb, mv};

use super::MoveGen;

#[derive(Deserialize)]
struct PerftPosition {
    depth: u8,
    nodes: u64,
    fen: String,
}

#[test]
fn test_perft() {
    let mut test_path = PathBuf::new();
    test_path.push(env!("CARGO_MANIFEST_DIR"));
    test_path.push("../resources/test/perft_positions.json");
    let test_file = fs::File::open(test_path).unwrap();
    let tests: Vec<PerftPosition> = serde_json::from_reader(test_file).unwrap();

    let move_gen = MoveGen::init();
    println!("Testing Perft...");
    for (i, test) in tests.iter().enumerate() {
        let position = Position::from_fen(&test.fen).unwrap();
        println!("Running test position {}...", i + 1);
        assert_eq!(
            move_gen.perft(position.clone(), test.depth),
            test.nodes,
            "running depth {} on position\n{}\n{position}",
            test.depth,
            test.fen,
        );
    }
    println!("All Perft test positions passed")
}

#[test]
fn correct_pawn_moves_in_starting_position() {
    let position = Position::new();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_pawn_moves::<false>(!bb!());
    let moves = state.moves;

    assert!(moves.contains(mv!(A2 -> A4)));
    assert!(moves.contains(mv!(F2 -> F4)));
    assert!(moves.contains(mv!(B2 -> B3)));
    assert!(moves.contains(mv!(H2 -> H3)));
    assert_eq!(moves.len(), 16);
}

#[test]
fn correct_forward_pawn_moves_for_black() {
    let fen = "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq - 0 1";
    let position = Position::from_fen(fen).unwrap();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_pawn_moves::<false>(!bb!());
    let moves = state.moves;

    assert!(moves.contains(mv!(A7 -> A5)));
    assert!(moves.contains(mv!(F7 -> F5)));
    assert!(moves.contains(mv!(B7 -> B6)));
    assert!(moves.contains(mv!(H7 -> H6)));
    assert_eq!(moves.len(), 16);
}

#[test]
fn correct_pawn_captures_for_white() {
    let fen = "rnbqkb1r/p1p1p1pp/7n/1p1p1pP1/P3P3/8/1PPP1P1P/RNBQKBNR w KQkq - 1 5";
    let position = Position::from_fen(fen).unwrap();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_pawn_moves::<false>(!bb!());
    let moves = state.moves;

    assert!(moves.contains(mv!(A4 x B5)));
    assert!(moves.contains(mv!(E4 x D5)));
    assert!(moves.contains(mv!(E4 x F5)));
    assert!(moves.contains(mv!(G5 x H6)));
}

#[test]
fn correct_pawn_captures_for_black() {
    let fen = "rnbqkbnr/p1p1p1pp/8/1p1p4/P1B1p1P1/5N2/1PPP1P1P/RNBQK2R b KQkq - 1 5";
    let position = Position::from_fen(fen).unwrap();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_pawn_moves::<false>(!bb!());
    let moves = state.moves;

    assert!(moves.contains(mv!(B5 x A4)));
    assert!(moves.contains(mv!(B5 x C4)));
    assert!(moves.contains(mv!(D5 x C4)));
    assert!(moves.contains(mv!(E4 x F3)));
}

#[test]
fn correct_en_passant() {
    let fen = "rnbqkb1r/p1p1pppp/5n2/1pPpP3/6P1/8/PP1P1P1P/RNBQKBNR w KQkq d6 0 6";
    let position = Position::from_fen(fen).unwrap();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_pawn_moves::<false>(!bb!());
    let moves = state.moves;

    assert!(moves.contains(mv!(C5 ep D6)));
    assert!(moves.contains(mv!(E5 ep D6)));
    assert!(!moves.contains(mv!(C5 ep B6)));
}

#[test]
fn correct_promotions() {
    let position =
        Position::from_fen("r1bqk2r/pP2bpPp/2n1p3/8/8/3P1P2/PPP4P/RNBQKBNR w KQ - 1 11").unwrap();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_pawn_moves::<false>(!bb!());
    let moves = state.moves;

    assert!(moves.contains(mv!(B7 x A8 n)));
    assert!(moves.contains(mv!(B7 -> B8 b)));
    assert!(moves.contains(mv!(B7 x C8 r)));
    assert!(moves.contains(mv!(B7 x C8 q)));
    assert!(moves.contains(mv!(G7 -> G8 q)));
    assert!(moves.contains(mv!(G7 x H8 q)));
}

#[test]
fn correct_knight_moves_for_white() {
    let fen = "rn1qkbnr/pp2pppp/8/3pN3/2p3b1/3P1P2/PPP1P1PP/RNBQKB1R w KQkq - 0 5";
    let position = Position::from_fen(fen).unwrap();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_non_pawn_moves::<false>(PieceKind::Knight, !bb!());
    let moves = state.moves;

    assert!(moves.contains(mv!(B1 -> A3)));
    assert!(moves.contains(mv!(B1 -> D2)));
    assert!(moves.contains(mv!(E5 x C4)));
    assert!(moves.contains(mv!(E5 -> G6)));
    assert!(moves.contains(mv!(E5 x G4)));
    assert_eq!(moves.len(), 9);
}

#[test]
fn correct_knight_moves_for_black() {
    let fen = "r2qkbnr/pp2pppp/2n5/P2pN3/2p3b1/3P1P2/1PP1P1PP/RNBQKB1R b KQkq - 0 6";
    let position = Position::from_fen(fen).unwrap();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_non_pawn_moves::<false>(PieceKind::Knight, !bb!());
    let moves = state.moves;

    assert!(moves.contains(mv!(C6 -> B8)));
    assert!(moves.contains(mv!(C6 x A5)));
    assert!(moves.contains(mv!(C6 x E5)));
    assert!(moves.contains(mv!(G8 -> F6)));
    assert!(moves.contains(mv!(G8 -> H6)));
    assert_eq!(moves.len(), 7);
}

#[test]
fn correct_non_castling_king_moves_for_white() {
    let fen = "r1bqkb1r/ppp1ppp1/2n5/2Pp4/4P2p/3Kn2P/PP1P1PP1/RNBQ1BNR w kq - 0 8";
    let position = Position::from_fen(fen).unwrap();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_non_pawn_moves::<false>(PieceKind::King, !bb!());
    let moves = state.moves;

    assert!(moves.contains(mv!(D3 x E3)));
    assert!(moves.contains(mv!(D3 -> E2)));
    assert!(moves.contains(mv!(D3 -> C3)));
    assert_eq!(moves.len(), 3);
}

#[test]
fn correct_non_castling_king_moves_for_black() {
    let fen = "rnb3nr/1p4bp/p1ppk1pP/q4NP1/P3pp2/1P6/2PPPP2/R1BQKBNR b KQ - 1 13";
    let position = Position::from_fen(fen).unwrap();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_non_pawn_moves::<false>(PieceKind::King, !bb!());
    let moves = state.moves;

    assert!(moves.contains(mv!(E6 -> D5)));
    assert!(moves.contains(mv!(E6 x F5)));
    assert!(moves.contains(mv!(E6 -> F7)));
    assert!(moves.contains(mv!(E6 -> E5)));
    assert!(moves.contains(mv!(E6 -> D7)));
    assert_eq!(moves.len(), 5);
}

#[test]
fn both_sides_castling_moves() {
    let fen = "rnbqkbnr/7p/ppppppp1/1B6/3PPB2/2NQ1N2/PPP2PPP/R3K2R w KQkq - 0 8";
    let position = Position::from_fen(fen).unwrap();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_castling_moves();
    let moves = state.moves;

    let king_side_count = moves.iter().filter(|&&m| m == mv!(O-O w)).count();
    let queen_side_count = moves.iter().filter(|&&m| m == mv!(O-O-O w)).count();

    assert_eq!(king_side_count, 1);
    assert_eq!(queen_side_count, 1);
}

#[test]
fn no_castling_through_pieces() {
    let fen = "rn2k2r/ppp2ppp/3q1n2/3pp3/1b1PPPb1/1PP1B1P1/P6P/RN1QKBNR b KQkq - 0 7";
    let position = Position::from_fen(fen).unwrap();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_castling_moves();
    let moves = state.moves;

    let king_side_count = moves.iter().filter(|&&m| m == mv!(O-O b)).count();

    assert_eq!(king_side_count, 1);
    assert!(!moves.contains(mv!(O-O-O b)));
}

#[test]
fn no_castling_through_attacks() {
    let fen = "r1bqkb1r/pppppppp/8/6B1/3PP2P/n2Q1Nn1/PPP2PP1/R3K2R w KQkq - 0 10";
    let position = Position::from_fen(fen).unwrap();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_castling_moves();
    let moves = state.moves;

    let queen_side_count = moves.iter().filter(|&&m| m == mv!(O-O-O w)).count();

    assert!(!moves.contains(mv!(O-O w)));
    assert_eq!(queen_side_count, 1);
}

#[test]
fn no_castling_when_in_check() {
    let fen = "r3k2r/ppp1bppp/2nq1N2/3p4/3PP3/2P5/PP2BPPP/RNBQK2R b KQkq - 0 8";
    let position = Position::from_fen(fen).unwrap();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_castling_moves();
    let moves = state.moves;

    assert!(!moves.contains(mv!(O-O b)));
    assert!(!moves.contains(mv!(O-O-O b)));
}

#[test]
fn correct_bishop_moves() {
    let fen = "rnbqkbnr/1ppp1pp1/p7/4p2p/1PB1P3/8/P1PP1PPP/RNBQK1NR w KQkq - 0 5";
    let position = Position::from_fen(fen).unwrap();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_non_pawn_moves::<false>(PieceKind::Bishop, !bb!());
    let moves = state.moves;

    assert!(moves.contains(mv!(C1 -> B2)));
    assert!(moves.contains(mv!(C4 -> B3)));
    assert!(moves.contains(mv!(C4 x F7)));
    assert!(moves.contains(mv!(C4 -> E2)));
    assert!(!moves.contains(mv!(C1 -> D2)));
    assert!(!moves.contains(mv!(C1 x D2)));
    assert_eq!(moves.len(), 11);
}

#[test]
fn correct_rook_moves() {
    let fen = "1nbqkbnr/1pppppp1/8/p2r4/4PPp1/3P3P/PPP1N3/RNBQKB1R b KQk - 1 7";
    let position = Position::from_fen(fen).unwrap();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_non_pawn_moves::<false>(PieceKind::Rook, !bb!());
    let moves = state.moves;

    assert!(moves.contains(mv!(D5 -> D6)));
    assert!(moves.contains(mv!(D5 -> B5)));
    assert!(moves.contains(mv!(D5 -> H5)));
    assert!(moves.contains(mv!(H8 x H3)));
    assert!(!moves.contains(mv!(D5 -> A5)));
    assert!(!moves.contains(mv!(D5 x A5)));
    assert_eq!(moves.len(), 14);
}

#[test]
fn correct_queen_moves() {
    let fen = "r1bqkbnr/p1pp1p1p/1pnPp1p1/8/3Q4/8/PPP1PPPP/RNB1KBNR w KQkq - 0 5";
    let position = Position::from_fen(fen).unwrap();
    let mut state = MoveGenState::new(&position, Tables::get());

    state.gen_non_pawn_moves::<false>(PieceKind::Queen, !bb!());
    let moves = state.moves;

    assert!(moves.contains(mv!(D4 -> D5)));
    assert!(moves.contains(mv!(D4 x H8)));
    assert!(moves.contains(mv!(D4 -> G4)));
    assert!(moves.contains(mv!(D4 -> E3)));
    assert!(moves.contains(mv!(D4 -> D2)));
    assert!(moves.contains(mv!(D4 -> C3)));
    assert!(moves.contains(mv!(D4 -> A4)));
    assert!(moves.contains(mv!(D4 x B6)));
    assert!(!moves.contains(mv!(D4 -> D6)));
    assert!(!moves.contains(mv!(D4 x D6)));
    assert_eq!(moves.len(), 19);
}

#[test]
fn checkers_correct_with_pawn() {
    let fen = "rnbqkbnr/ppp1pp1p/6p1/8/3pP3/4K3/PPPP1PPP/RNBQ1BNR w kq - 0 4";
    let position = Position::from_fen(fen).unwrap();
    let state = MoveGenState::new(&position, Tables::get());

    assert_eq!(state.checkers(), bb!(D4));
}

#[test]
fn checkers_correct_with_knight() {
    let fen = "rnbq1bnr/pppppkpp/5p2/6N1/8/7P/PPPPPPP1/RNBQKB1R b KQ - 2 3";
    let position = Position::from_fen(fen).unwrap();
    let state = MoveGenState::new(&position, Tables::get());

    assert_eq!(state.checkers(), bb!(G5));
}

#[test]
fn checkers_correct_with_bishop() {
    let fen = "rnbqk1nr/pppp1ppp/4p3/8/1b1P4/6P1/PPP1PP1P/RNBQKBNR w KQkq - 1 3";
    let position = Position::from_fen(fen).unwrap();
    let state = MoveGenState::new(&position, Tables::get());

    assert_eq!(state.checkers(), bb!(B4));
}

#[test]
fn checkers_correct_with_rook() {
    let fen = "rnbqkbnr/pppp1pp1/7p/8/5p1P/4R3/PPPPP1P1/RNBQKBN1 b Qkq - 1 4";
    let position = Position::from_fen(fen).unwrap();
    let state = MoveGenState::new(&position, Tables::get());

    assert_eq!(state.checkers(), bb!(E3));
}

#[test]
fn checkers_correct_with_queen() {
    let fen = "rnb1kbnr/pp1ppppp/8/q1p5/3PP3/8/PPP2PPP/RNBQKBNR w KQkq - 1 3";
    let position = Position::from_fen(fen).unwrap();
    let state = MoveGenState::new(&position, Tables::get());

    assert_eq!(state.checkers(), bb!(A5));
}

#[test]
fn checkers_correct_with_double_check() {
    let fen = "rnbqkbnr/2p2Ppp/pp6/8/8/8/PPPPQPPP/RNB1KBNR b KQkq - 0 5";
    let position = Position::from_fen(fen).unwrap();
    let state = MoveGenState::new(&position, Tables::get());

    assert_eq!(state.checkers(), bb!(E2, F7));
}

#[test]
fn check_avoidance_with_captures_blocks_and_dodges() {
    let fen = "rnbqk1nr/1ppp1p2/p5pp/3Pp3/1b1QP3/P7/1PP2PPP/RNB1KBNR w KQkq - 1 6";
    let position = Position::from_fen(fen).unwrap();
    let move_gen = MoveGen::init();

    let moves = move_gen.gen_all_moves(&position);

    // Captures
    assert!(moves.contains(mv!(A3 x B4)));
    assert!(moves.contains(mv!(D4 x B4)));
    // Blocks
    assert!(moves.contains(mv!(C2 -> C3)));
    assert!(moves.contains(mv!(B1 -> C3)));
    assert!(moves.contains(mv!(B1 -> D2)));
    assert!(moves.contains(mv!(C1 -> D2)));
    assert!(moves.contains(mv!(D4 -> C3)));
    assert!(moves.contains(mv!(D4 -> D2)));
    // Dodges
    assert!(moves.contains(mv!(E1 -> D1)));
    assert!(moves.contains(mv!(E1 -> E2)));
    // These are the only moves
    assert_eq!(moves.len(), 10);
}

#[test]
fn check_avoidance_with_en_passant_capture_and_king_capture() {
    let fen = "rnbq1bnr/pppp1ppp/8/4k3/Q1PPp2P/6P1/PP2PP2/RNB1KBNR b KQ d3 0 6";
    let position = Position::from_fen(fen).unwrap();
    let move_gen = MoveGen::init();

    let moves = move_gen.gen_all_moves(&position);

    assert!(moves.contains(mv!(E4 ep D3)));
    assert!(moves.contains(mv!(E5 x D4)));
    assert_eq!(moves.len(), 6);
}

#[test]
fn pin_rays_correct() {
    let fen = "N3kbn1/p2q1p2/2p1n3/1b1Pp2p/2P3p1/3K1Pr1/1P1P2PP/RNBQ1BNR w - - 16 22";
    let position = Position::from_fen(fen).unwrap();
    let state = MoveGenState::new(&position, Tables::get());

    assert_eq!(state.pin_rays, bb!(C4, B5, D4, D5, D6, D7, E3, F3, G3));
}

#[test]
fn pinned_pawns_can_only_move_along_pin_rays() {
    let fen = "N3kbn1/p2q1p2/2p1n3/1b1Pp2p/2P3p1/3K1Pr1/1P1P2PP/RNBQ1BNR w - - 16 22";
    let position = Position::from_fen(fen).unwrap();
    let move_gen = MoveGen::init();

    let moves = move_gen.gen_all_moves(&position);

    assert!(moves.contains(mv!(C4 x B5)));
    assert!(moves.contains(mv!(D5 -> D6)));
    assert!(!moves.contains(mv!(C4 -> C5)));
    assert!(!moves.contains(mv!(D5 x C6)));
    assert!(!moves.contains(mv!(D5 x E6)));
    assert!(!moves.contains(mv!(F3 -> F4)));
    assert!(!moves.contains(mv!(F3 x G4)));
}

#[test]
fn cant_capture_en_passant_due_to_pin() {
    let fen = "rnbq1bnr/pppp1p2/P5p1/8/N1RPpk1p/1P5P/1BP1PPP1/3QKBNR b K d3 0 12";
    let position = Position::from_fen(fen).unwrap();
    let move_gen = MoveGen::init();

    let moves = move_gen.gen_all_moves(&position);

    assert!(!moves.contains(mv!(E4 ep D3)));
}

#[test]
fn pinned_knights_cant_move() {
    let fen = "8/4k3/3nn3/8/1B2R3/8/3K4/8 b - - 0 1";
    let position = Position::from_fen(fen).unwrap();
    let move_gen = MoveGen::init();

    let moves = move_gen.gen_all_moves(&position);
    assert!(!moves.contains(mv!(D6 x E4)));
    assert!(!moves.contains(mv!(E6 -> C5)));
    assert_eq!(moves.len(), 6);
}

#[test]
fn pinned_sliding_pieces_can_only_move_along_pin_rays() {
    let fen = "8/4k1b1/8/4B3/3KRq2/8/5Q2/6q1 w - - 0 1";
    let position = Position::from_fen(fen).unwrap();
    let move_gen = MoveGen::init();

    let moves = move_gen.gen_all_moves(&position);

    assert!(moves.contains(mv!(E5 -> F6)));
    assert!(moves.contains(mv!(E5 x G7)));
    assert!(moves.contains(mv!(E4 x F4)));
    assert!(moves.contains(mv!(F2 -> E3)));
    assert!(moves.contains(mv!(F2 x G1)));
    assert!(!moves.contains(mv!(E5 x F4)));
    assert!(!moves.contains(mv!(E5 -> C7)));
    assert!(!moves.contains(mv!(E4 -> E2)));
    assert!(!moves.contains(mv!(F2 x F4)));
    assert!(!moves.contains(mv!(F2 -> B2)));
    assert!(!moves.contains(mv!(F2 -> E1)));
    assert!(!moves.contains(mv!(F2 -> H4)));
}

#[test]
fn gen_captures_works() {
    let fen = "2k5/1p1p2R1/4q3/1QpPn3/8/3N2B1/8/7K w - c6 0 1";
    let position = Position::from_fen(fen).unwrap();
    let move_gen = MoveGen::init();

    let moves = move_gen.gen_captures(&position);

    assert!(moves.contains(mv!(D5 x E6)));
    assert!(moves.contains(mv!(D5 ep C6)));
    assert!(moves.contains(mv!(D3 x C5)));
    assert!(moves.contains(mv!(D3 x E5)));
    assert!(moves.contains(mv!(G3 x E5)));
    assert!(moves.contains(mv!(G7 x D7)));
    assert!(moves.contains(mv!(B5 x B7)));
    assert!(moves.contains(mv!(B5 x D7)));
    assert!(moves.contains(mv!(B5 x C5)));
    assert_eq!(moves.len(), 9);
}

// Test positions added to fix bugs in the move generator

#[test]
fn test_position_1() {
    let fen = "4k2r/1b4bq/8/8/8/8/7B/rR2K2R w Kk - 0 1";
    let position = Position::from_fen(fen).unwrap();
    let move_gen = MoveGen::init();

    let moves = move_gen.gen_all_moves(&position);

    assert!(moves.contains(mv!(H1 -> G1)));
    assert!(moves.contains(mv!(H1 -> F1)));
}

#[test]
fn test_position_2() {
    let fen = "rnbqkb1r/pppppppp/8/8/4n3/3P4/PPPKPPPP/RNBQ1BNR w kq - 3 3";
    let position = Position::from_fen(fen).unwrap();
    let move_gen = MoveGen::init();

    let moves = move_gen.gen_all_moves(&position);

    assert!(moves.contains(mv!(D2 -> E3)));
    assert!(moves.contains(mv!(D2 -> E1)));
    assert!(moves.contains(mv!(D3 x E4)));
}
