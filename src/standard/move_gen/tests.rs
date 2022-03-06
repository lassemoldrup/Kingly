use crate::bb;
use crate::framework::MoveGen as MoveGenTrait;
use crate::framework::moves::{Move, MoveList};
use crate::framework::piece::PieceKind;
use crate::framework::square::Square;
use crate::standard::{Bitboard, MoveGen, Position};
use crate::standard::tables::Tables;

// Unit testing for MoveGen

fn get_move_gen() -> MoveGen {
    MoveGen::new(Tables::get())
}

#[test]
fn correct_pawn_moves_in_starting_position() {
    let position = Position::new();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_pawn_moves::<false>(&position, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::new_regular(Square::A2, Square::A4)));
    assert!(moves.contains(Move::new_regular(Square::F2, Square::F4)));
    assert!(moves.contains(Move::new_regular(Square::B2, Square::B3)));
    assert!(moves.contains(Move::new_regular(Square::H2, Square::H3)));
    assert_eq!(moves.len(), 16);
}

#[test]
fn correct_forward_pawn_moves_for_black() {
    let position = Position::from_fen("rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq - 0 1").unwrap();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_pawn_moves::<false>(&position, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::new_regular(Square::A7, Square::A5)));
    assert!(moves.contains(Move::new_regular(Square::F7, Square::F5)));
    assert!(moves.contains(Move::new_regular(Square::B7, Square::B6)));
    assert!(moves.contains(Move::new_regular(Square::H7, Square::H6)));
    assert_eq!(moves.len(), 16);
}

#[test]
fn correct_pawn_captures_for_white() {
    let position = Position::from_fen("rnbqkb1r/p1p1p1pp/7n/1p1p1pP1/P3P3/8/1PPP1P1P/RNBQKBNR w KQkq - 1 5").unwrap();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_pawn_moves::<false>(&position, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::new_regular(Square::A4, Square::B5)));
    assert!(moves.contains(Move::new_regular(Square::E4, Square::D5)));
    assert!(moves.contains(Move::new_regular(Square::E4, Square::F5)));
    assert!(moves.contains(Move::new_regular(Square::G5, Square::H6)));
}

#[test]
fn correct_pawn_captures_for_black() {
    let position = Position::from_fen("rnbqkbnr/p1p1p1pp/8/1p1p4/P1B1p1P1/5N2/1PPP1P1P/RNBQK2R b KQkq - 1 5").unwrap();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_pawn_moves::<false>(&position, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::new_regular(Square::B5, Square::A4)));
    assert!(moves.contains(Move::new_regular(Square::B5, Square::C4)));
    assert!(moves.contains(Move::new_regular(Square::D5, Square::C4)));
    assert!(moves.contains(Move::new_regular(Square::E4, Square::F3)));
}

#[test]
fn correct_en_passant() {
    let position = Position::from_fen("rnbqkb1r/p1p1pppp/5n2/1pPpP3/6P1/8/PP1P1P1P/RNBQKBNR w KQkq d6 0 6").unwrap();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_pawn_moves::<false>(&position, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::new_en_passant(Square::C5, Square::D6)));
    assert!(moves.contains(Move::new_en_passant(Square::E5, Square::D6)));
    assert!(!moves.contains(Move::new_en_passant(Square::C5, Square::B6)));
}

#[test]
fn correct_promotions() {
    let position = Position::from_fen("r1bqk2r/pP2bpPp/2n1p3/8/8/3P1P2/PPP4P/RNBQKBNR w KQ - 1 11").unwrap();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_pawn_moves::<false>(&position, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::new_promotion(Square::B7, Square::A8, PieceKind::Knight)));
    assert!(moves.contains(Move::new_promotion(Square::B7, Square::B8, PieceKind::Bishop)));
    assert!(moves.contains(Move::new_promotion(Square::B7, Square::C8, PieceKind::Rook)));
    assert!(moves.contains(Move::new_promotion(Square::B7, Square::C8, PieceKind::Queen)));
    assert!(moves.contains(Move::new_promotion(Square::G7, Square::G8, PieceKind::Queen)));
    assert!(moves.contains(Move::new_promotion(Square::G7, Square::H8, PieceKind::Queen)));
}

#[test]
fn correct_knight_moves_for_white() {
    let position = Position::from_fen("rn1qkbnr/pp2pppp/8/3pN3/2p3b1/3P1P2/PPP1P1PP/RNBQKB1R w KQkq - 0 5").unwrap();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_non_pawn_moves::<false>(&position, PieceKind::Knight, !Bitboard::new(), Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::new_regular(Square::B1, Square::A3)));
    assert!(moves.contains(Move::new_regular(Square::B1, Square::D2)));
    assert!(moves.contains(Move::new_regular(Square::E5, Square::C4)));
    assert!(moves.contains(Move::new_regular(Square::E5, Square::G6)));
    assert!(moves.contains(Move::new_regular(Square::E5, Square::G4)));
    assert_eq!(moves.len(), 9);
}

#[test]
fn correct_knight_moves_for_black() {
    let position = Position::from_fen("r2qkbnr/pp2pppp/2n5/P2pN3/2p3b1/3P1P2/1PP1P1PP/RNBQKB1R b KQkq - 0 6").unwrap();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_non_pawn_moves::<false>(&position, PieceKind::Knight, !Bitboard::new(), Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::new_regular(Square::C6, Square::B8)));
    assert!(moves.contains(Move::new_regular(Square::C6, Square::A5)));
    assert!(moves.contains(Move::new_regular(Square::C6, Square::E5)));
    assert!(moves.contains(Move::new_regular(Square::G8, Square::F6)));
    assert!(moves.contains(Move::new_regular(Square::G8, Square::H6)));
    assert_eq!(moves.len(), 7);
}

#[test]
fn correct_non_castling_king_moves_for_white() {
    let position = Position::from_fen("r1bqkbnr/ppp1ppp1/2n5/2Pp4/4P2p/3K4/PP1P1PPP/RNBQ1BNR w kq - 0 6").unwrap();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_non_pawn_moves::<false>(&position, PieceKind::King, !Bitboard::new(), Bitboard::new(), move_gen.gen_danger_sqs(&position), &mut moves);

    assert!(moves.contains(Move::new_regular(Square::D3, Square::E3)));
    assert!(moves.contains(Move::new_regular(Square::D3, Square::E2)));
    assert!(moves.contains(Move::new_regular(Square::D3, Square::C3)));
    assert!(moves.contains(Move::new_regular(Square::D3, Square::C2)));
    assert_eq!(moves.len(), 4);
}

#[test]
fn correct_non_castling_king_moves_for_black() {
    let position = Position::from_fen("rnb3nr/pp4bp/2ppk1pP/q5P1/P3pp2/1P2N3/2PPPP2/R1BQKBNR b KQ - 0 12").unwrap();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_non_pawn_moves::<false>(&position, PieceKind::King, !Bitboard::new(), Bitboard::new(), move_gen.gen_danger_sqs(&position), &mut moves);

    assert!(moves.contains(Move::new_regular(Square::E6, Square::E7)));
    assert!(moves.contains(Move::new_regular(Square::E6, Square::F7)));
    assert!(moves.contains(Move::new_regular(Square::E6, Square::E5)));
    assert!(moves.contains(Move::new_regular(Square::E6, Square::D7)));
    assert_eq!(moves.len(), 4);
}

#[test]
fn both_sides_castling_moves() {
    let position = Position::from_fen("rnbqkbnr/7p/ppppppp1/1B6/3PPB2/2NQ1N2/PPP2PPP/R3K2R w KQkq - 0 8").unwrap();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_castling_moves(&position, move_gen.gen_danger_sqs(&position), &mut moves);

    let king_side_count = moves.iter()
        .filter(|&&m| m == Move::new_castling(Square::E1, Square::G1))
        .count();
    
    let queen_side_count = moves.iter()
        .filter(|&&m| m == Move::new_castling(Square::E1, Square::C1))
        .count();

    assert_eq!(king_side_count, 1);
    assert_eq!(queen_side_count, 1);
}

#[test]
fn no_castling_through_pieces() {
    let position = Position::from_fen("rn2k2r/ppp2ppp/3q1n2/3pp3/1b1PPPb1/1PP1B1P1/P6P/RN1QKBNR b KQkq - 0 7").unwrap();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_castling_moves(&position, move_gen.gen_danger_sqs(&position), &mut moves);

    let king_side_count = moves.iter()
        .filter(|&&m| m == Move::new_castling(Square::E8, Square::G8))
        .count();

    assert_eq!(king_side_count, 1);
    assert!(!moves.contains(Move::new_castling(Square::E8, Square::C8)));
}


#[test]
fn no_castling_through_attacks() {
    let position = Position::from_fen("r1bqkb1r/pppppppp/8/6B1/3PP2P/n2Q1Nn1/PPP2PP1/R3K2R w KQkq - 0 10").unwrap();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_castling_moves(&position, move_gen.gen_danger_sqs(&position), &mut moves);

    let queen_side_count = moves.iter()
        .filter(|&&m| m == Move::new_castling(Square::E1, Square::C1))
        .count();

    assert!(!moves.contains(Move::new_castling(Square::E1, Square::G1)));
    assert_eq!(queen_side_count, 1);
}

#[test]
fn no_castling_when_in_check() {
    let position = Position::from_fen("r3k2r/ppp1bppp/2nq1N2/3p4/3PP3/2P5/PP2BPPP/RNBQK2R b KQkq - 0 8").unwrap();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_castling_moves(&position, move_gen.gen_danger_sqs(&position), &mut moves);

    assert!(!moves.contains(Move::new_castling(Square::E8, Square::G8)));
    assert!(!moves.contains(Move::new_castling(Square::E8, Square::C8)));
}

#[test]
fn correct_bishop_moves() {
    let position = Position::from_fen("rnbqkbnr/1ppp1pp1/p7/4p2p/1PB1P3/8/P1PP1PPP/RNBQK1NR w KQkq - 0 5").unwrap();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_non_pawn_moves::<false>(&position, PieceKind::Bishop, !Bitboard::new(), Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::new_regular(Square::C1, Square::B2)));
    assert!(moves.contains(Move::new_regular(Square::C4, Square::B3)));
    assert!(moves.contains(Move::new_regular(Square::C4, Square::F7)));
    assert!(moves.contains(Move::new_regular(Square::C4, Square::E2)));
    assert!(!moves.contains(Move::new_regular(Square::C1, Square::D2)));
    assert_eq!(moves.len(), 11);
}

#[test]
fn correct_rook_moves() {
    let position = Position::from_fen("1nbqkbnr/1pppppp1/8/p2r4/4PPp1/3P3P/PPP1N3/RNBQKB1R b KQk - 1 7").unwrap();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_non_pawn_moves::<false>(&position, PieceKind::Rook, !Bitboard::new(), Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::new_regular(Square::D5, Square::D6)));
    assert!(moves.contains(Move::new_regular(Square::D5, Square::B5)));
    assert!(moves.contains(Move::new_regular(Square::D5, Square::H5)));
    assert!(moves.contains(Move::new_regular(Square::H8, Square::H3)));
    assert!(!moves.contains(Move::new_regular(Square::D5, Square::A5)));
    assert_eq!(moves.len(), 14);
}

#[test]
fn correct_queen_moves() {
    let position = Position::from_fen("r1bqkbnr/p1pp1p1p/1pnPp1p1/8/3Q4/8/PPP1PPPP/RNB1KBNR w KQkq - 0 5").unwrap();
    let move_gen = get_move_gen();

    let mut moves = MoveList::new();
    move_gen.gen_non_pawn_moves::<false>(&position, PieceKind::Queen, !Bitboard::new(), Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::new_regular(Square::D4, Square::D5)));
    assert!(moves.contains(Move::new_regular(Square::D4, Square::H8)));
    assert!(moves.contains(Move::new_regular(Square::D4, Square::G4)));
    assert!(moves.contains(Move::new_regular(Square::D4, Square::E3)));
    assert!(moves.contains(Move::new_regular(Square::D4, Square::D2)));
    assert!(moves.contains(Move::new_regular(Square::D4, Square::C3)));
    assert!(moves.contains(Move::new_regular(Square::D4, Square::A4)));
    assert!(moves.contains(Move::new_regular(Square::D4, Square::B6)));
    assert!(!moves.contains(Move::new_regular(Square::D4, Square::D6)));
    assert_eq!(moves.len(), 19);
}

#[test]
fn checkers_correct_with_pawn() {
    let position = Position::from_fen("rnbqkbnr/ppp1pp1p/6p1/8/3pP3/4K3/PPPP1PPP/RNBQ1BNR w kq - 0 4").unwrap();
    let move_gen = get_move_gen();

    assert_eq!(move_gen.checkers(&position), bb!(Square::D4));
}

#[test]
fn checkers_correct_with_knight() {
    let position = Position::from_fen("rnbq1bnr/pppppkpp/5p2/6N1/8/7P/PPPPPPP1/RNBQKB1R b KQ - 2 3").unwrap();
    let move_gen = get_move_gen();

    assert_eq!(move_gen.checkers(&position), bb!(Square::G5));
}

#[test]
fn checkers_correct_with_bishop() {
    let position = Position::from_fen("rnbqk1nr/pppp1ppp/4p3/8/1b1P4/6P1/PPP1PP1P/RNBQKBNR w KQkq - 1 3").unwrap();
    let move_gen = get_move_gen();

    assert_eq!(move_gen.checkers(&position), bb!(Square::B4));
}

#[test]
fn checkers_correct_with_rook() {
    let position = Position::from_fen("rnbqkbnr/pppp1pp1/7p/8/5p1P/4R3/PPPPP1P1/RNBQKBN1 b Qkq - 1 4").unwrap();
    let move_gen = get_move_gen();

    assert_eq!(move_gen.checkers(&position), bb!(Square::E3));
}

#[test]
fn checkers_correct_with_queen() {
    let position = Position::from_fen("rnb1kbnr/pp1ppppp/8/q1p5/3PP3/8/PPP2PPP/RNBQKBNR w KQkq - 1 3").unwrap();
    let move_gen = get_move_gen();

    assert_eq!(move_gen.checkers(&position), bb!(Square::A5));
}

#[test]
fn checkers_correct_with_double_check() {
    let position = Position::from_fen("rnbqkbnr/2p2Ppp/pp6/8/8/8/PPPPQPPP/RNB1KBNR b KQkq - 0 5").unwrap();
    let move_gen = get_move_gen();

    assert_eq!(move_gen.checkers(&position), bb!(Square::E2, Square::F7));
}

#[test]
fn check_avoidance_with_captures_blocks_and_dodges() {
    let position = Position::from_fen("rnbqk1nr/1ppp1p2/p5pp/3Pp3/1b1QP3/P7/1PP2PPP/RNB1KBNR w KQkq - 1 6").unwrap();
    let move_gen = get_move_gen();

    let moves = move_gen.gen_all_moves(&position);

    // Captures
    assert!(moves.contains(Move::new_regular(Square::A3, Square::B4)));
    assert!(moves.contains(Move::new_regular(Square::D4, Square::B4)));
    // Blocks
    assert!(moves.contains(Move::new_regular(Square::C2, Square::C3)));
    assert!(moves.contains(Move::new_regular(Square::B1, Square::C3)));
    assert!(moves.contains(Move::new_regular(Square::B1, Square::D2)));
    assert!(moves.contains(Move::new_regular(Square::C1, Square::D2)));
    assert!(moves.contains(Move::new_regular(Square::D4, Square::C3)));
    assert!(moves.contains(Move::new_regular(Square::D4, Square::D2)));
    // Dodges
    assert!(moves.contains(Move::new_regular(Square::E1, Square::D1)));
    assert!(moves.contains(Move::new_regular(Square::E1, Square::E2)));
    // These are the only moves
    assert_eq!(moves.len(), 10);
}

#[test]
fn check_avoidance_with_en_passant_capture_and_king_capture() {
    let position = Position::from_fen("rnbq1bnr/pppp1ppp/8/4k3/Q1PPp2P/6P1/PP2PP2/RNB1KBNR b KQ d3 0 6").unwrap();
    let move_gen = get_move_gen();

    let moves = move_gen.gen_all_moves(&position);

    assert!(moves.contains(Move::new_en_passant(Square::E4, Square::D3)));
    assert!(moves.contains(Move::new_regular(Square::E5, Square::D4)));
    assert_eq!(moves.len(), 6);
}

#[test]
fn pin_rays_correct() {
    let position = Position::from_fen("N3kbn1/p2q1p2/2p1n3/1b1Pp2p/2P3p1/3K1Pr1/1P1P2PP/RNBQ1BNR w - - 16 22").unwrap();
    let move_gen = get_move_gen();

    let pin_rays = move_gen.pin_rays(&position);
    use Square::*;
    assert_eq!(pin_rays, bb!(C4, B5, D4, D5, D6, D7, E3, F3, G3));
}

#[test]
fn pinned_pawns_can_only_move_along_pin_rays() {
    let position = Position::from_fen("N3kbn1/p2q1p2/2p1n3/1b1Pp2p/2P3p1/3K1Pr1/1P1P2PP/RNBQ1BNR w - - 16 22").unwrap();
    let move_gen = get_move_gen();

    let moves = move_gen.gen_all_moves(&position);

    assert!(moves.contains(Move::new_regular(Square::C4, Square::B5)));
    assert!(moves.contains(Move::new_regular(Square::D5, Square::D6)));
    assert!(!moves.contains(Move::new_regular(Square::C4, Square::C5)));
    assert!(!moves.contains(Move::new_regular(Square::D5, Square::C6)));
    assert!(!moves.contains(Move::new_regular(Square::D5, Square::E6)));
    assert!(!moves.contains(Move::new_regular(Square::F3, Square::F4)));
    assert!(!moves.contains(Move::new_regular(Square::F3, Square::G4)));
}

#[test]
fn cant_capture_en_passant_due_to_pin() {
    let position = Position::from_fen("rnbq1bnr/pppp1p2/P5p1/8/N1RPpk1p/1P5P/1BP1PPP1/3QKBNR b K d3 0 12").unwrap();
    let move_gen = get_move_gen();

    let moves = move_gen.gen_all_moves(&position);

    assert!(!moves.contains(Move::new_en_passant(Square::E4, Square::D3)));
}

#[test]
fn pinned_knights_cant_move() {
    let position = Position::from_fen("8/4k3/3nn3/8/1B2R3/8/3K4/8 b - - 0 1").unwrap();
    let move_gen = get_move_gen();

    let moves = move_gen.gen_all_moves(&position);
    assert!(!moves.contains(Move::new_regular(Square::D6, Square::E4)));
    assert!(!moves.contains(Move::new_regular(Square::E6, Square::C5)));
    assert_eq!(moves.len(), 6);
}

#[test]
fn pinned_sliding_pieces_can_only_move_along_pin_rays() {
    let position = Position::from_fen("8/4k1b1/8/4B3/3KRq2/8/5Q2/6q1 w - - 0 1").unwrap();
    let move_gen = get_move_gen();

    let moves = move_gen.gen_all_moves(&position);

    assert!(moves.contains(Move::new_regular(Square::E5, Square::F6)));
    assert!(moves.contains(Move::new_regular(Square::E5, Square::G7)));
    assert!(moves.contains(Move::new_regular(Square::E4, Square::F4)));
    assert!(moves.contains(Move::new_regular(Square::F2, Square::E3)));
    assert!(moves.contains(Move::new_regular(Square::F2, Square::G1)));
    assert!(!moves.contains(Move::new_regular(Square::E5, Square::F4)));
    assert!(!moves.contains(Move::new_regular(Square::E5, Square::C7)));
    assert!(!moves.contains(Move::new_regular(Square::E4, Square::E2)));
    assert!(!moves.contains(Move::new_regular(Square::F2, Square::F4)));
    assert!(!moves.contains(Move::new_regular(Square::F2, Square::B2)));
    assert!(!moves.contains(Move::new_regular(Square::F2, Square::E1)));
    assert!(!moves.contains(Move::new_regular(Square::F2, Square::H4)));
}

#[test]
fn gen_captures_works() {
    let position = Position::from_fen("2k5/1p1p2R1/4q3/1QpPn3/8/3N2B1/8/7K w - c6 0 1").unwrap();
    let move_gen = get_move_gen();

    let moves = move_gen.gen_captures(&position);

    assert!(moves.contains(Move::new_regular(Square::D5, Square::E6)));
    assert!(moves.contains(Move::new_en_passant(Square::D5, Square::C6)));
    assert!(moves.contains(Move::new_regular(Square::D3, Square::C5)));
    assert!(moves.contains(Move::new_regular(Square::D3, Square::E5)));
    assert!(moves.contains(Move::new_regular(Square::G3, Square::E5)));
    assert!(moves.contains(Move::new_regular(Square::G7, Square::D7)));
    assert!(moves.contains(Move::new_regular(Square::B5, Square::B7)));
    assert!(moves.contains(Move::new_regular(Square::B5, Square::D7)));
    assert!(moves.contains(Move::new_regular(Square::B5, Square::C5)));
    assert_eq!(moves.len(), 9);
}

// Test positions added to fix bugs in the move generator

#[test]
fn test_position_1() {
    let position = Position::from_fen("4k2r/1b4bq/8/8/8/8/7B/rR2K2R w Kk - 0 1").unwrap();
    let move_gen = get_move_gen();

    let moves = move_gen.gen_all_moves(&position);

    assert!(moves.contains(Move::new_regular(Square::H1, Square::G1)));
    assert!(moves.contains(Move::new_regular(Square::H1, Square::F1)));
}

#[test]
fn test_position_2() {
    let position = Position::from_fen("rnbqkb1r/pppppppp/8/8/4n3/3P4/PPPKPPPP/RNBQ1BNR w kq - 3 3").unwrap();
    let move_gen = get_move_gen();

    let moves = move_gen.gen_all_moves(&position);

    assert!(moves.contains(Move::new_regular(Square::D2, Square::E3)));
    assert!(moves.contains(Move::new_regular(Square::D2, Square::E1)));
    assert!(moves.contains(Move::new_regular(Square::D3, Square::E4)));
}