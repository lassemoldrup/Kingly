use crate::types::{Move, Piece, PieceType, Color, bin_vec::*, Square};
use crate::bb;
use crate::position::Position;
use crate::types::bitboard::*;
use arrayvec::ArrayVec;
use crate::types::square_map::SquareMap;
use crate::lookup_table::Lookup;

mod magics;
pub use magics::{init_magics, all_lookups_init};
pub mod perft;

#[derive(Eq, PartialEq)]
enum Type {
    All,
    Captures,
    Quiet,
}

// needed bcs of compiler bug, delete later
pub unsafe fn gentest(pos: &Position, moves: &mut ArrayVec<[Move; 256]>) -> u8 {
    generate_all::<{Color::White}>(pos, moves)
}

/// Generates all legal moves
/// # Safety
/// `moves` has to have enough space
/// All lookup tables have to be initialized
pub unsafe fn generate_all<const US: Color>(pos: &Position, moves: &mut ArrayVec<[Move; 256]>) -> u8 {
    debug_assert!(moves.is_empty());
    debug_assert!(all_lookups_init());

    const TYPE: Type = Type::All;
    if US == Color::White {
        generate_pawn_moves::<WhitePawns, {TYPE}>(pos, moves);
    } else {
        generate_pawn_moves::<BlackPawns, {TYPE}>(pos, moves);
    }
    generate_moves::<{Piece(US, PieceType::Knight)}, {TYPE}>(pos, moves);
    generate_moves::<{Piece(US, PieceType::Bishop)}, {TYPE}>(pos, moves);
    generate_moves::<{Piece(US, PieceType::Rook)},   {TYPE}>(pos, moves);
    generate_moves::<{Piece(US, PieceType::Queen)},  {TYPE}>(pos, moves);
    generate_moves::<{Piece(US, PieceType::King)},   {TYPE}>(pos, moves);
    generate_castling::<{US}>(pos, moves);

    moves.len() as u8
}

trait Pawns {
    const PIECE: Piece;
    const UP: BinVec;
    const UP_LEFT: BinVec;
    const UP_RIGHT: BinVec;
    const OUR_4TH: Bitboard;
    const OUR_8TH: Bitboard;
}
struct WhitePawns;
struct BlackPawns;
impl Pawns for WhitePawns {
    const PIECE: Piece = Piece(Color::White, PieceType::Pawn);
    const UP: BinVec = NORTH;
    const UP_LEFT: BinVec = NORTH_WEST;
    const UP_RIGHT: BinVec = NORTH_EAST;
    const OUR_4TH: Bitboard = RANK_4_BB;
    const OUR_8TH: Bitboard = RANK_8_BB;
}
impl Pawns for BlackPawns {
    const PIECE: Piece = Piece(Color::Black, PieceType::Pawn);
    const UP: BinVec = SOUTH;
    const UP_LEFT: BinVec = SOUTH_EAST;
    const UP_RIGHT: BinVec = SOUTH_WEST;
    const OUR_4TH: Bitboard = RANK_5_BB;
    const OUR_8TH: Bitboard = RANK_1_BB;
}

/// Generates pawn moves
/// # Safety
/// `moves` has to have enough space
unsafe fn generate_pawn_moves<P: Pawns, const TYPE: Type>(pos: &Position, moves: &mut ArrayVec<[Move; 256]>) {
    let pawns = pos.get_piece_bb(P::PIECE);

    unsafe fn add_regular(vec: BinVec, board: Bitboard, moves: &mut ArrayVec<[Move; 256]>) {
        for sq in board {
            moves.push_unchecked(Move::Regular(sq.add_unchecked(-vec), sq));
        }
    }

    unsafe fn add_promotions(vec: BinVec, promo: Bitboard, moves: &mut ArrayVec<[Move; 256]>) {
        for sq in promo {
            let from = sq.add_unchecked(-vec);
            moves.push_unchecked(Move::Promotion(from, sq, PieceType::Queen));
            moves.push_unchecked(Move::Promotion(from, sq, PieceType::Rook));
            moves.push_unchecked(Move::Promotion(from, sq, PieceType::Bishop));
            moves.push_unchecked(Move::Promotion(from, sq, PieceType::Knight));
        }
    }

    if TYPE != Type::Captures {
        let not_occupied = !pos.get_occupied();
        let forward = shift::<{P::UP}>(pawns) & not_occupied;
        let forward2 = shift::<{P::UP}>(forward) & not_occupied & P::OUR_4TH;
        let no_promo = forward & !P::OUR_8TH;
        add_regular(P::UP, no_promo, moves);
        add_regular(P::UP * 2, forward2, moves);
        if TYPE != Type::Quiet {
            let promo = forward & P::OUR_8TH;
            add_promotions(P::UP, promo, moves);
        }
    }

    if TYPE != Type::Quiet {
        let left = shift::<{P::UP_LEFT}>(pawns);
        let right = shift::<{P::UP_RIGHT}>(pawns);
        let opp_pawns = pos.get_color_bb(!P::PIECE.color());
        let left_atk = left & opp_pawns;
        let right_atk = right & opp_pawns;
        // en passant
        if let Some(sq) = pos.get_en_passant_sq() {
            let en_passant = Bitboard::from_sq(sq);
            let en_passant_left = left & en_passant;
            let en_passant_right = right & en_passant;
            if en_passant_left != Bitboard::EMPTY {
                moves.push_unchecked(Move::EnPassant(sq.add_unchecked(-P::UP_LEFT), sq));
            }
            if en_passant_right != Bitboard::EMPTY {
                moves.push_unchecked(Move::EnPassant(sq.add_unchecked(-P::UP_RIGHT), sq));
            }
        }
        let left_atk_no_promo = left_atk & !P::OUR_8TH;
        let right_atk_no_promo = right_atk & !P::OUR_8TH;
        add_regular(P::UP_LEFT, left_atk_no_promo, moves);
        add_regular(P::UP_RIGHT, right_atk_no_promo, moves);

        // maybe check for promotion
        if true {
            let left_atk_promo = left_atk & P::OUR_8TH;
            let right_atk_promo = right_atk & P::OUR_8TH;
            add_promotions(P::UP_LEFT, left_atk_promo, moves);
            add_promotions(P::UP_RIGHT, right_atk_promo, moves);
        }
    }
}

/// Generates castling moves
/// # Safety
/// `moves` has to have space
unsafe fn generate_castling<const US: Color>(pos: &Position, moves: &mut ArrayVec<[Move; 256]>) {
    match US {
        Color::White => {
            const OCCUPANCY_MASK: Bitboard = bb!(Square::B1, Square::C1, Square::D1, Square::F1, Square::G1);
            let castling = (OCCUPANCY_MASK.set_diff(pos.get_occupied()) | bb!(Square::E1))
                .set_diff(pos.get_attack_bb(!US)) & pos.get_castling_rights();
            const KINGSIDE: Bitboard = bb!(Square::E1, Square::F1, Square::G1);
            const QUEENSIDE: Bitboard = bb!(Square::B1, Square::C1, Square::D1, Square::E1);
            if castling & KINGSIDE == KINGSIDE {
                moves.push_unchecked(Move::Castling(Square::E1, Square::G1));
            }
            if castling & QUEENSIDE == QUEENSIDE {
                moves.push_unchecked(Move::Castling(Square::E1, Square::C1));
            }
        },
        Color::Black => {
            const OCCUPANCY_MASK: Bitboard = bb!(Square::B8, Square::C8, Square::D8, Square::F8, Square::G8);
            let castling = (OCCUPANCY_MASK.set_diff(pos.get_occupied()) | bb!(Square::E8))
                .set_diff(pos.get_attack_bb(!US)) & pos.get_castling_rights();
            const KINGSIDE: Bitboard = bb!(Square::E8, Square::F8, Square::G8);
            const QUEENSIDE: Bitboard = bb!(Square::B8, Square::C8, Square::D8, Square::E8);
            if castling & KINGSIDE == KINGSIDE {
                moves.push_unchecked(Move::Castling(Square::E8, Square::G8));
            }
            if castling & QUEENSIDE == QUEENSIDE {
                moves.push_unchecked(Move::Castling(Square::E8, Square::C8));
            }
        }
    }
}

static WHITE_PAWN_ATTACKS: Lookup<SquareMap<Bitboard>> = Lookup::new(SquareMap::new(Bitboard::EMPTY));
static BLACK_PAWN_ATTACKS: Lookup<SquareMap<Bitboard>> = Lookup::new(SquareMap::new(Bitboard::EMPTY));
static KNIGHT_ATTACKS: Lookup<SquareMap<Bitboard>> = Lookup::new(SquareMap::new(Bitboard::EMPTY));
static KING_ATTACKS: Lookup<SquareMap<Bitboard>> = Lookup::new(SquareMap::new(Bitboard::EMPTY));

/// Generates regular moves for all but pawns
/// # Safety
/// `moves` has to have enough space
/// All lookup tables have to be initialized
unsafe fn generate_moves<const PCE: Piece, const TYPE: Type>(pos: &Position, moves: &mut ArrayVec<[Move; 256]>) {
    if PCE.kind() == PieceType::Pawn {
        panic!("generate_moves shouldn't be used with pawns");
    }
    for from in pos.get_piece_bb(PCE) {
        let attacks = generate_attacks::<{PCE}>(pos, from);
        let mut legal = match TYPE {
            Type::All       =>  attacks.set_diff(pos.get_color_bb(PCE.color())),
            Type::Quiet     =>  attacks.set_diff(pos.get_occupied()),
            Type::Captures  =>  attacks & pos.get_occupied(),
        };
        if PCE.kind() == PieceType::King {
            legal = legal.set_diff(pos.get_attack_bb(!PCE.color()));
        }
        for to in legal {
            moves.push_unchecked(Move::Regular(from, to));
        }
    }
}

pub fn init_non_sliders() {
    fn init(vecs: Vec<BinVec>, table: &Lookup<SquareMap<Bitboard>>) {
        for sq in Square::A1.range_to(Square::H8) {
            let bb = vecs.iter().fold(Bitboard::EMPTY, |acc, vec| {
                match sq + *vec {
                    Some(dest) => acc | Bitboard::from_sq(dest),
                    None => acc,
                }
            });
            table.set(sq, bb);
        }
        table.set_init();
    }
    let white_pawn_vecs = vec![BinVec(7), BinVec(9)];
    let black_pawn_vecs = vec![BinVec(-7), BinVec(-9)];
    let knight_vecs = vec![BinVec(6), BinVec(15), BinVec(17), BinVec(10),
                           BinVec(-6), BinVec(-15), BinVec(-17), BinVec(-10)];
    let king_vecs = vec![NORTH, NORTH_EAST, EAST, SOUTH_EAST, SOUTH, SOUTH_WEST, WEST, NORTH_WEST];
    init(white_pawn_vecs, &WHITE_PAWN_ATTACKS);
    init(black_pawn_vecs, &BLACK_PAWN_ATTACKS);
    init(knight_vecs, &KNIGHT_ATTACKS);
    init(king_vecs, &KING_ATTACKS);
}

/// Generates a bitboard with all squares attacked from a given piece
/// # Safety
/// All lookup tables have to be initialized
pub unsafe fn generate_attacks<const PCE: Piece>(pos: &Position, sq: Square) -> Bitboard {
    match PCE.kind() {
        PieceType::Pawn => match PCE.color() {
            Color::White => WHITE_PAWN_ATTACKS.get(sq),
            Color::Black => BLACK_PAWN_ATTACKS.get(sq),
        },
        PieceType::Knight => KNIGHT_ATTACKS.get(sq),
        PieceType::Bishop | PieceType::Rook | PieceType::Queen =>
            magics::slider_attacks::<{PCE.kind()}>(pos.get_occupied(), sq),
        PieceType::King   => KING_ATTACKS.get(sq),
    }
}

/// Wrapper for `generates_attacks` that doesn't require constant a constant piece
/// TODO: horrible design, maybe figure something better out
/// # Safety
/// See `generate_attacks`
pub unsafe fn generate_attacks_no_const(pos: &Position, sq: Square, pce: Piece) -> Bitboard {
    match (pce.color(), pce.kind()) {
        (Color::White, PieceType::Pawn) => generate_attacks::<{Piece(Color::White, PieceType::Pawn)}>(pos, sq),
        (Color::White, PieceType::Knight) => generate_attacks::<{Piece(Color::White, PieceType::Knight)}>(pos, sq),
        (Color::White, PieceType::Bishop) => generate_attacks::<{Piece(Color::White, PieceType::Bishop)}>(pos, sq),
        (Color::White, PieceType::Rook) => generate_attacks::<{Piece(Color::White, PieceType::Rook)}>(pos, sq),
        (Color::White, PieceType::Queen) => generate_attacks::<{Piece(Color::White, PieceType::Queen)}>(pos, sq),
        (Color::White, PieceType::King) => generate_attacks::<{Piece(Color::White, PieceType::King)}>(pos, sq),
        (Color::Black, PieceType::Pawn) => generate_attacks::<{Piece(Color::Black, PieceType::Pawn)}>(pos, sq),
        (Color::Black, PieceType::Knight) => generate_attacks::<{Piece(Color::Black, PieceType::Knight)}>(pos, sq),
        (Color::Black, PieceType::Bishop) => generate_attacks::<{Piece(Color::Black, PieceType::Bishop)}>(pos, sq),
        (Color::Black, PieceType::Rook) => generate_attacks::<{Piece(Color::Black, PieceType::Rook)}>(pos, sq),
        (Color::Black, PieceType::Queen) => generate_attacks::<{Piece(Color::Black, PieceType::Queen)}>(pos, sq),
        (Color::Black, PieceType::King) => generate_attacks::<{Piece(Color::Black, PieceType::King)}>(pos, sq),
    }
}