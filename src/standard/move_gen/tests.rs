use crate::bb;
use crate::framework::moves::{Move, MoveList};
use crate::framework::piece::PieceKind;
use crate::framework::Side;
use crate::framework::square::Square;
use crate::standard::bitboard::Bitboard;
use crate::standard::move_gen::MoveGen;
use crate::standard::position::Position;

// Unit testing for MoveGen

fn get_move_gen(position: &Position) -> MoveGen {
    let mut move_gen = MoveGen::new();
    move_gen.gen_danger_sqs(&position);
    move_gen
}

#[test]
fn correct_pawn_moves_in_starting_position() {
    let position = Position::new();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_pawn_moves(&position, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::Regular(Square::A2, Square::A4)));
    assert!(moves.contains(Move::Regular(Square::F2, Square::F4)));
    assert!(moves.contains(Move::Regular(Square::B2, Square::B3)));
    assert!(moves.contains(Move::Regular(Square::H2, Square::H3)));
    assert_eq!(moves.len(), 16);
}

#[test]
fn correct_forward_pawn_moves_for_black() {
    let position = Position::from_fen("rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq - 0 1").unwrap();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_pawn_moves(&position, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::Regular(Square::A7, Square::A5)));
    assert!(moves.contains(Move::Regular(Square::F7, Square::F5)));
    assert!(moves.contains(Move::Regular(Square::B7, Square::B6)));
    assert!(moves.contains(Move::Regular(Square::H7, Square::H6)));
    assert_eq!(moves.len(), 16);
}

#[test]
fn correct_pawn_captures_for_white() {
    let position = Position::from_fen("rnbqkb1r/p1p1p1pp/7n/1p1p1pP1/P3P3/8/1PPP1P1P/RNBQKBNR w KQkq - 1 5").unwrap();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_pawn_moves(&position, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::Regular(Square::A4, Square::B5)));
    assert!(moves.contains(Move::Regular(Square::E4, Square::D5)));
    assert!(moves.contains(Move::Regular(Square::E4, Square::F5)));
    assert!(moves.contains(Move::Regular(Square::G5, Square::H6)));
}

#[test]
fn correct_pawn_captures_for_black() {
    let position = Position::from_fen("rnbqkbnr/p1p1p1pp/8/1p1p4/P1B1p1P1/5N2/1PPP1P1P/RNBQK2R b KQkq - 1 5").unwrap();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_pawn_moves(&position, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::Regular(Square::B5, Square::A4)));
    assert!(moves.contains(Move::Regular(Square::B5, Square::C4)));
    assert!(moves.contains(Move::Regular(Square::D5, Square::C4)));
    assert!(moves.contains(Move::Regular(Square::E4, Square::F3)));
}

#[test]
fn correct_en_passant() {
    let position = Position::from_fen("rnbqkb1r/p1p1pppp/5n2/1pPpP3/6P1/8/PP1P1P1P/RNBQKBNR w KQkq d6 0 6").unwrap();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_pawn_moves(&position, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::EnPassant(Square::C5, Square::D6)));
    assert!(moves.contains(Move::EnPassant(Square::E5, Square::D6)));
    assert!(!moves.contains(Move::EnPassant(Square::C5, Square::B6)));
}

#[test]
fn correct_promotions() {
    let position = Position::from_fen("r1bqk2r/pP2bpPp/2n1p3/8/8/3P1P2/PPP4P/RNBQKBNR w KQ - 1 11").unwrap();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_pawn_moves(&position, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::Promotion(Square::B7, Square::A8, PieceKind::Knight)));
    assert!(moves.contains(Move::Promotion(Square::B7, Square::B8, PieceKind::Bishop)));
    assert!(moves.contains(Move::Promotion(Square::B7, Square::C8, PieceKind::Rook)));
    assert!(moves.contains(Move::Promotion(Square::B7, Square::C8, PieceKind::Queen)));
    assert!(moves.contains(Move::Promotion(Square::G7, Square::G8, PieceKind::Queen)));
    assert!(moves.contains(Move::Promotion(Square::G7, Square::H8, PieceKind::Queen)));
    assert!(!moves.contains(Move::Promotion(Square::B7, Square::B8, PieceKind::King)));
    assert!(!moves.contains(Move::Promotion(Square::B7, Square::B8, PieceKind::Pawn)));
}

#[test]
fn correct_knight_moves_for_white() {
    let position = Position::from_fen("rn1qkbnr/pp2pppp/8/3pN3/2p3b1/3P1P2/PPP1P1PP/RNBQKB1R w KQkq - 0 5").unwrap();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_non_pawn_moves(&position, PieceKind::Knight, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::Regular(Square::B1, Square::A3)));
    assert!(moves.contains(Move::Regular(Square::B1, Square::D2)));
    assert!(moves.contains(Move::Regular(Square::E5, Square::C4)));
    assert!(moves.contains(Move::Regular(Square::E5, Square::G6)));
    assert!(moves.contains(Move::Regular(Square::E5, Square::G4)));
    assert_eq!(moves.len(), 9);
}

#[test]
fn correct_knight_moves_for_black() {
    let position = Position::from_fen("r2qkbnr/pp2pppp/2n5/P2pN3/2p3b1/3P1P2/1PP1P1PP/RNBQKB1R b KQkq - 0 6").unwrap();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_non_pawn_moves(&position, PieceKind::Knight, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::Regular(Square::C6, Square::B8)));
    assert!(moves.contains(Move::Regular(Square::C6, Square::A5)));
    assert!(moves.contains(Move::Regular(Square::C6, Square::E5)));
    assert!(moves.contains(Move::Regular(Square::G8, Square::F6)));
    assert!(moves.contains(Move::Regular(Square::G8, Square::H6)));
    assert_eq!(moves.len(), 7);
}

#[test]
fn correct_non_castling_king_moves_for_white() {
    let position = Position::from_fen("r1bqkbnr/ppp1ppp1/2n5/2Pp4/4P2p/3K4/PP1P1PPP/RNBQ1BNR w kq - 0 6").unwrap();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_non_pawn_moves(&position, PieceKind::King, !Bitboard::new(),Bitboard::new(),  &mut moves);

    assert!(moves.contains(Move::Regular(Square::D3, Square::E3)));
    assert!(moves.contains(Move::Regular(Square::D3, Square::E2)));
    assert!(moves.contains(Move::Regular(Square::D3, Square::C3)));
    assert!(moves.contains(Move::Regular(Square::D3, Square::C2)));
    assert_eq!(moves.len(), 4);
}

#[test]
fn correct_non_castling_king_moves_for_black() {
    let position = Position::from_fen("rnb3nr/pp4bp/2ppk1pP/q5P1/P3pp2/1P2N3/2PPPP2/R1BQKBNR b KQ - 0 12").unwrap();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_non_pawn_moves(&position, PieceKind::King, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::Regular(Square::E6, Square::E7)));
    assert!(moves.contains(Move::Regular(Square::E6, Square::F7)));
    assert!(moves.contains(Move::Regular(Square::E6, Square::E5)));
    assert!(moves.contains(Move::Regular(Square::E6, Square::D7)));
    assert_eq!(moves.len(), 4);
}

#[test]
fn both_sides_castling_moves() {
    let position = Position::from_fen("rnbqkbnr/7p/ppppppp1/1B6/3PPB2/2NQ1N2/PPP2PPP/R3K2R w KQkq - 0 8").unwrap();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_castling_moves(&position, &mut moves);

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
    let position = Position::from_fen("rn2k2r/ppp2ppp/3q1n2/3pp3/1b1PPPb1/1PP1B1P1/P6P/RN1QKBNR b KQkq - 0 7").unwrap();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_castling_moves(&position, &mut moves);

    let king_side_count = moves.iter()
        .filter(|&&m| m == Move::Castling(Side::KingSide))
        .count();

    assert_eq!(king_side_count, 1);
    assert!(!moves.contains(Move::Castling(Side::QueenSide)));
}


#[test]
fn no_castling_through_attacks() {
    let position = Position::from_fen("r1bqkb1r/pppppppp/8/6B1/3PP2P/n2Q1Nn1/PPP2PP1/R3K2R w KQkq - 0 10").unwrap();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_castling_moves(&position, &mut moves);

    let queen_side_count = moves.iter()
        .filter(|&&m| m == Move::Castling(Side::QueenSide))
        .count();

    assert!(!moves.contains(Move::Castling(Side::KingSide)));
    assert_eq!(queen_side_count, 1);
}

#[test]
fn no_castling_when_in_check() {
    let position = Position::from_fen("r3k2r/ppp1bppp/2nq1N2/3p4/3PP3/2P5/PP2BPPP/RNBQK2R b KQkq - 0 8").unwrap();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_castling_moves(&position, &mut moves);

    assert!(!moves.contains(Move::Castling(Side::KingSide)));
    assert!(!moves.contains(Move::Castling(Side::QueenSide)));
}

#[test]
fn bishop_masks_initialized_correctly() {
    let position = Position::new();
    let move_gen = get_move_gen(&position);

    use Square::*;
    assert_eq!(move_gen.bishop_masks[A8], bb!(B7, C6, D5, E4, F3, G2));
    assert_eq!(move_gen.bishop_masks[B2], bb!(C3, D4, E5, F6, G7));
    assert_eq!(move_gen.bishop_masks[D5], bb!(E6, F7, E4, F3, G2, C4, B3, C6, B7));
}

#[test]
fn rook_masks_initialized_correctly() {
    let position = Position::new();
    let move_gen = get_move_gen(&position);

    use Square::*;
    assert_eq!(move_gen.rook_masks[A8], bb!(B8, C8, D8, E8, F8, G8, A7, A6, A5, A4, A3, A2));
    assert_eq!(move_gen.rook_masks[B2], bb!(B3, B4, B5, B6, B7, C2, D2, E2, F2, G2));
    assert_eq!(move_gen.rook_masks[D5], bb!(E5, F5, G5, D4, D3, D2, C5, B5, D6, D7));
}

#[test]
fn correct_bishop_moves() {
    let position = Position::from_fen("rnbqkbnr/1ppp1pp1/p7/4p2p/1PB1P3/8/P1PP1PPP/RNBQK1NR w KQkq - 0 5").unwrap();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_non_pawn_moves(&position, PieceKind::Bishop, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::Regular(Square::C1, Square::B2)));
    assert!(moves.contains(Move::Regular(Square::C4, Square::B3)));
    assert!(moves.contains(Move::Regular(Square::C4, Square::F7)));
    assert!(moves.contains(Move::Regular(Square::C4, Square::E2)));
    assert!(!moves.contains(Move::Regular(Square::C1, Square::D2)));
    assert_eq!(moves.len(), 11);
}

#[test]
fn correct_rook_moves() {
    let position = Position::from_fen("1nbqkbnr/1pppppp1/8/p2r4/4PPp1/3P3P/PPP1N3/RNBQKB1R b KQk - 1 7").unwrap();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_non_pawn_moves(&position, PieceKind::Rook, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::Regular(Square::D5, Square::D6)));
    assert!(moves.contains(Move::Regular(Square::D5, Square::B5)));
    assert!(moves.contains(Move::Regular(Square::D5, Square::H5)));
    assert!(moves.contains(Move::Regular(Square::H8, Square::H3)));
    assert!(!moves.contains(Move::Regular(Square::D5, Square::A5)));
    assert_eq!(moves.len(), 14);
}

#[test]
fn correct_queen_moves() {
    let position = Position::from_fen("r1bqkbnr/p1pp1p1p/1pnPp1p1/8/3Q4/8/PPP1PPPP/RNB1KBNR w KQkq - 0 5").unwrap();
    let move_gen = get_move_gen(&position);

    let mut moves = MoveList::new();
    move_gen.gen_non_pawn_moves(&position, PieceKind::Queen, !Bitboard::new(), Bitboard::new(), &mut moves);

    assert!(moves.contains(Move::Regular(Square::D4, Square::D5)));
    assert!(moves.contains(Move::Regular(Square::D4, Square::H8)));
    assert!(moves.contains(Move::Regular(Square::D4, Square::G4)));
    assert!(moves.contains(Move::Regular(Square::D4, Square::E3)));
    assert!(moves.contains(Move::Regular(Square::D4, Square::D2)));
    assert!(moves.contains(Move::Regular(Square::D4, Square::C3)));
    assert!(moves.contains(Move::Regular(Square::D4, Square::A4)));
    assert!(moves.contains(Move::Regular(Square::D4, Square::B6)));
    assert!(!moves.contains(Move::Regular(Square::D4, Square::D6)));
    assert_eq!(moves.len(), 19);
}

#[test]
fn checkers_correct_with_pawn() {
    let position = Position::from_fen("rnbqkbnr/ppp1pp1p/6p1/8/3pP3/4K3/PPPP1PPP/RNBQ1BNR w kq - 0 4").unwrap();
    let move_gen = get_move_gen(&position);

    assert_eq!(move_gen.checkers(&position), bb!(Square::D4));
}

#[test]
fn checkers_correct_with_knight() {
    let position = Position::from_fen("rnbq1bnr/pppppkpp/5p2/6N1/8/7P/PPPPPPP1/RNBQKB1R b KQ - 2 3").unwrap();
    let move_gen = get_move_gen(&position);

    assert_eq!(move_gen.checkers(&position), bb!(Square::G5));
}

#[test]
fn checkers_correct_with_bishop() {
    let position = Position::from_fen("rnbqk1nr/pppp1ppp/4p3/8/1b1P4/6P1/PPP1PP1P/RNBQKBNR w KQkq - 1 3").unwrap();
    let move_gen = get_move_gen(&position);

    assert_eq!(move_gen.checkers(&position), bb!(Square::B4));
}

#[test]
fn checkers_correct_with_rook() {
    let position = Position::from_fen("rnbqkbnr/pppp1pp1/7p/8/5p1P/4R3/PPPPP1P1/RNBQKBN1 b Qkq - 1 4").unwrap();
    let move_gen = get_move_gen(&position);

    assert_eq!(move_gen.checkers(&position), bb!(Square::E3));
}

#[test]
fn checkers_correct_with_queen() {
    let position = Position::from_fen("rnb1kbnr/pp1ppppp/8/q1p5/3PP3/8/PPP2PPP/RNBQKBNR w KQkq - 1 3").unwrap();
    let move_gen = get_move_gen(&position);

    assert_eq!(move_gen.checkers(&position), bb!(Square::A5));
}

#[test]
fn checkers_correct_with_double_check() {
    let position = Position::from_fen("rnbqkbnr/2p2Ppp/pp6/8/8/8/PPPPQPPP/RNB1KBNR b KQkq - 0 5").unwrap();
    let move_gen = get_move_gen(&position);

    assert_eq!(move_gen.checkers(&position), bb!(Square::E2, Square::F7));
}

#[test]
fn check_avoidance_with_captures_blocks_and_dodges() {
    let position = Position::from_fen("rnbqk1nr/1ppp1p2/p5pp/3Pp3/1b1QP3/P7/1PP2PPP/RNB1KBNR w KQkq - 1 6").unwrap();
    let mut move_gen = MoveGen::new();

    let moves = move_gen.gen_all_moves(&position);

    // Captures
    assert!(moves.contains(Move::Regular(Square::A3, Square::B4)));
    assert!(moves.contains(Move::Regular(Square::D4, Square::B4)));
    // Blocks
    assert!(moves.contains(Move::Regular(Square::C2, Square::C3)));
    assert!(moves.contains(Move::Regular(Square::B1, Square::C3)));
    assert!(moves.contains(Move::Regular(Square::B1, Square::D2)));
    assert!(moves.contains(Move::Regular(Square::C1, Square::D2)));
    assert!(moves.contains(Move::Regular(Square::D4, Square::C3)));
    assert!(moves.contains(Move::Regular(Square::D4, Square::D2)));
    // Dodges
    assert!(moves.contains(Move::Regular(Square::E1, Square::D1)));
    assert!(moves.contains(Move::Regular(Square::E1, Square::E2)));
    // These are the only moves
    assert_eq!(moves.len(), 10);
}

#[test]
fn check_avoidance_with_en_passant_capture_and_king_capture() {
    let position = Position::from_fen("rnbq1bnr/pppp1ppp/8/4k3/Q1PPp2P/6P1/PP2PP2/RNB1KBNR b KQ d3 0 6").unwrap();
    let mut move_gen = MoveGen::new();

    let moves = move_gen.gen_all_moves(&position);

    assert!(moves.contains(Move::EnPassant(Square::E4, Square::D3)));
    assert!(moves.contains(Move::Regular(Square::E5, Square::D4)));
    assert_eq!(moves.len(), 6);
}

#[test]
fn line_through_initialized_correctly() {
    let move_gen = MoveGen::new();

    use Square::*;
    assert_eq!(move_gen.line_through[B1][B5], bb!(B1, B2, B3, B4, B5, B6, B7, B8));
    assert_eq!(move_gen.line_through[F8][C5], bb!(A3, B4, C5, D6, E7, F8));
    assert_eq!(move_gen.line_through[D4][E4], bb!(A4, B4, C4, D4, E4, F4, G4, H4));
    assert_eq!(move_gen.line_through[A8][H1], bb!(A8, B7, C6, D5, E4, F3, G2, H1));
    assert_eq!(move_gen.line_through[C4][D6], Bitboard::new());
}

#[test]
fn ray_to_initialized_correctly() {
    let move_gen = MoveGen::new();

    assert_eq!(move_gen.ray_to[Square::B1][Square::B5], bb!(Square::B2, Square::B3, Square::B4, Square::B5));
    assert_eq!(move_gen.ray_to[Square::F8][Square::C5], bb!(Square::E7, Square::D6, Square::C5));
    assert_eq!(move_gen.ray_to[Square::C4][Square::D6], Bitboard::new());
}

#[test]
fn pin_rays_correct() {
    let position = Position::from_fen("N3kbn1/p2q1p2/2p1n3/1b1Pp2p/2P3p1/3K1Pr1/1P1P2PP/RNBQ1BNR w - - 16 22").unwrap();
    let move_gen = MoveGen::new();

    let pin_rays = move_gen.pin_rays(&position);
    use Square::*;
    assert_eq!(pin_rays, bb!(C4, B5, D4, D5, D6, D7, E3, F3, G3));
}

#[test]
fn pinned_pawns_can_only_move_along_pin_rays() {
    let position = Position::from_fen("N3kbn1/p2q1p2/2p1n3/1b1Pp2p/2P3p1/3K1Pr1/1P1P2PP/RNBQ1BNR w - - 16 22").unwrap();
    let mut move_gen = MoveGen::new();

    let moves = move_gen.gen_all_moves(&position);

    assert!(moves.contains(Move::Regular(Square::C4, Square::B5)));
    assert!(moves.contains(Move::Regular(Square::D5, Square::D6)));
    assert!(!moves.contains(Move::Regular(Square::C4, Square::C5)));
    assert!(!moves.contains(Move::Regular(Square::D5, Square::C6)));
    assert!(!moves.contains(Move::Regular(Square::D5, Square::E6)));
    assert!(!moves.contains(Move::Regular(Square::F3, Square::F4)));
    assert!(!moves.contains(Move::Regular(Square::F3, Square::G4)));
}

#[test]
fn cant_capture_en_passant_due_to_pin() {
    let position = Position::from_fen("rnbq1bnr/pppp1p2/P5p1/8/N1RPpk1p/1P5P/1BP1PPP1/3QKBNR b K d3 0 12").unwrap();
    let mut move_gen = MoveGen::new();

    let moves = move_gen.gen_all_moves(&position);

    assert!(!moves.contains(Move::EnPassant(Square::E4, Square::D3)));
}

#[test]
fn pinned_knights_cant_move() {
    let position = Position::from_fen("8/4k3/3nn3/8/1B2R3/8/3K4/8 b - - 0 1").unwrap();
    let mut move_gen = MoveGen::new();

    let moves = move_gen.gen_all_moves(&position);
    assert!(!moves.contains(Move::Regular(Square::D6, Square::E4)));
    assert!(!moves.contains(Move::Regular(Square::E6, Square::C5)));
    assert_eq!(moves.len(), 6);
}

#[test]
fn pinned_sliding_pieces_can_only_move_along_pin_rays() {
    let position = Position::from_fen("8/4k1b1/8/4B3/3KRq2/8/5Q2/6q1 w - - 0 1").unwrap();
    let mut move_gen = MoveGen::new();

    let moves = move_gen.gen_all_moves(&position);

    assert!(moves.contains(Move::Regular(Square::E5, Square::F6)));
    assert!(moves.contains(Move::Regular(Square::E5, Square::G7)));
    assert!(moves.contains(Move::Regular(Square::E4, Square::F4)));
    assert!(moves.contains(Move::Regular(Square::F2, Square::E3)));
    assert!(moves.contains(Move::Regular(Square::F2, Square::G1)));
    assert!(!moves.contains(Move::Regular(Square::E5, Square::F4)));
    assert!(!moves.contains(Move::Regular(Square::E5, Square::C7)));
    assert!(!moves.contains(Move::Regular(Square::E4, Square::E2)));
    assert!(!moves.contains(Move::Regular(Square::F2, Square::F4)));
    assert!(!moves.contains(Move::Regular(Square::F2, Square::B2)));
    assert!(!moves.contains(Move::Regular(Square::F2, Square::E1)));
    assert!(!moves.contains(Move::Regular(Square::F2, Square::H4)));
}