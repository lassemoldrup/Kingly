use crate::standard::position::Position;
use arrayvec::ArrayVec;
use crate::framework::moves::{Move, MoveList};
use crate::framework::color::Color;
use crate::framework::direction::Direction;
use crate::framework::piece::{PieceKind, Piece};
use crate::framework::square_map::SquareMap;
use crate::framework::square_vec::SquareVec;
use crate::standard::bitboard::Bitboard;
use crate::bb;
use crate::framework::square::Square;
use crate::framework::{CastlingRights, Side};
use std::convert::TryFrom;
use std::iter::repeat;
use bitintr::{Pdep, Pext};
use take_until::TakeUntilExt;

#[cfg(test)]
mod tests;

// TODO: maybe use const fns and statics for these lookup tables
pub struct MoveGen {
    danger_sqs: Bitboard,
    white_pawn_attacks: SquareMap<Bitboard>,
    black_pawn_attacks: SquareMap<Bitboard>,
    knight_attacks: SquareMap<Bitboard>,
    king_attacks: SquareMap<Bitboard>,
    bishop_masks: SquareMap<Bitboard>, // Relevant occupancy squares for bishop attacks
    rook_masks: SquareMap<Bitboard>, // Same for rooks
    slider_attacks: Vec<Bitboard>,
    bishop_offsets: SquareMap<usize>,
    rook_offsets: SquareMap<usize>,
}

impl MoveGen {
    pub fn new() -> Self {
        let white_pawn_attacks = Self::init_white_pawn_attacks();
        let black_pawn_attacks = Self::init_black_pawn_attacks();
        let knight_attacks = Self::init_knight_attacks();
        let king_attacks = Self::init_king_attacks();
        let bishop_masks = Self::init_bishop_masks();
        let rook_masks = Self::init_rook_masks();
        let slider_attacks = Vec::new();

        let mut move_gen = MoveGen {
            danger_sqs: Bitboard::new(),
            white_pawn_attacks,
            black_pawn_attacks,
            knight_attacks,
            king_attacks,
            bishop_masks,
            rook_masks,
            slider_attacks,
            bishop_offsets: SquareMap::new([0; 64]),
            rook_offsets: SquareMap::new([0; 64]),
        };

        move_gen.init_slider_attacks();
        move_gen
    }

    fn init_white_pawn_attacks() -> SquareMap<Bitboard> {
        let move_vecs = vec![SquareVec(1, 1), SquareVec(1, -1)];

        Self::init_step_attacks(move_vecs)
    }

    fn init_black_pawn_attacks() -> SquareMap<Bitboard> {
        let move_vecs = vec![SquareVec(-1, 1), SquareVec(-1, -1)];

        Self::init_step_attacks(move_vecs)
    }

    fn init_knight_attacks() -> SquareMap<Bitboard> {
        let move_vecs = vec![
            SquareVec(2, 1), SquareVec(1, 2), SquareVec(-1, 2), SquareVec(-2, 1),
            SquareVec(-2, -1), SquareVec(-1, -2), SquareVec(1, -2), SquareVec(2, -1),
        ];

        Self::init_step_attacks(move_vecs)
    }

    fn init_king_attacks() -> SquareMap<Bitboard> {
        let move_vecs = vec![
            SquareVec(1, 0), SquareVec(1, 1), SquareVec(0, 1), SquareVec(-1, 1),
            SquareVec(-1, 0), SquareVec(-1, -1), SquareVec(0, -1), SquareVec(1, -1),
        ];

        Self::init_step_attacks(move_vecs)
    }

    fn init_step_attacks(move_vecs: Vec<SquareVec>) -> SquareMap<Bitboard> {
        let mut table = SquareMap::new([Bitboard::new(); 64]);

        for (sq, bb) in table.iter_mut() {
            *bb = move_vecs.iter().fold(Bitboard::new(), |acc, vec| {
                match sq + *vec {
                    Some(dest) => acc.add_sq(dest),
                    None => acc,
                }
            });
        }

        table
    }

    const TO_SQ: fn((u8, u8)) -> Square = |(r, f)| Square::try_from(8 * r + f).unwrap();

    fn gen_bishop_attacks_slow(sq: Square, occ: Bitboard) -> Bitboard {
        let mut atk_bb = Bitboard::new();

        let rank = sq.rank();
        let file = sq.file();

        let sq_occ = |sq: &Square| occ.contains(*sq);

        let north_east = (rank+1..8).zip(file+1..8)
            .map(Self::TO_SQ)
            .take_until(sq_occ);
        let south_east = (0..rank).rev().zip(file+1..8)
            .map(Self::TO_SQ)
            .take_until(sq_occ);
        let south_west = (0..rank).rev().zip((0..file).rev())
            .map(Self::TO_SQ)
            .take_until(sq_occ);
        let north_west = (rank+1..8).zip((0..file).rev())
            .map(Self::TO_SQ)
            .take_until(sq_occ);

        for sq in north_east.chain(south_east).chain(south_west).chain(north_west) {
            atk_bb = atk_bb.add_sq(sq);
        }

        atk_bb
    }

    fn gen_rook_attacks_slow(sq: Square, occ: Bitboard) -> Bitboard {
        let mut atk_bb = Bitboard::new();

        let rank = sq.rank();
        let file = sq.file();

        let sq_occ = |sq: &Square| occ.contains(*sq);

        let north = (rank+1..8).zip(repeat(file))
            .map(Self::TO_SQ)
            .take_until(sq_occ);
        let east = repeat(rank).zip(file+1..8)
            .map(Self::TO_SQ)
            .take_until(sq_occ);
        let south = ((0..rank).rev()).zip(repeat(file))
            .map(Self::TO_SQ)
            .take_until(sq_occ);
        let west = repeat(rank).zip((0..file).rev())
            .map(Self::TO_SQ)
            .take_until(sq_occ);

        for sq in north.chain(east).chain(south).chain(west) {
            atk_bb = atk_bb.add_sq(sq);
        }

        atk_bb
    }


    fn init_bishop_masks() -> SquareMap<Bitboard> {
        let mut table = SquareMap::new([Bitboard::new(); 64]);

        for (sq, bb) in table.iter_mut() {
            *bb = Self::gen_bishop_attacks_slow(sq, Bitboard::new())
                - Bitboard::RANKS[0]
                - Bitboard::RANKS[7]
                - Bitboard::FILES[0]
                - Bitboard::FILES[7];
        }

        table
    }

    fn init_rook_masks() -> SquareMap<Bitboard> {
        let mut table = SquareMap::new([Bitboard::new(); 64]);

        for (sq, bb) in table.iter_mut() {
            *bb = Self::gen_rook_attacks_slow(sq, Bitboard::new());
            if sq.rank() != 0 {
                *bb = *bb - Bitboard::RANKS[0];
            }
            if sq.rank() != 7 {
                *bb = *bb - Bitboard::RANKS[7];
            }
            if sq.file() != 0 {
                *bb = *bb - Bitboard::FILES[0];
            }
            if sq.file() != 7 {
                *bb = *bb - Bitboard::FILES[7];
            }
        }

        table
    }

    fn init_slider_attacks(&mut self) {
        let mut table_idx = 0;

        for sq in Square::iter() {
            let count = 1 << self.bishop_masks[sq].len();

            for occ in 0..count as u64 {
                let occ_bb = occ.pdep(self.bishop_masks[sq].into()).into();
                let atk_bb = Self::gen_bishop_attacks_slow(sq, occ_bb);
                self.slider_attacks.push(atk_bb);
            }

            self.bishop_offsets[sq] = table_idx;
            table_idx += count;

            let count = 1 << self.rook_masks[sq].len();

            for occ in 0..count as u64 {
                let occ_bb = occ.pdep(self.rook_masks[sq].into()).into();
                let atk_bb = Self::gen_rook_attacks_slow(sq, occ_bb);
                self.slider_attacks.push(atk_bb);
            }

            self.rook_offsets[sq] = table_idx;
            table_idx += count;
        }
    }

    fn gen_pawn_moves(&self, position: &Position, moves: &mut MoveList) {
        let pawns = position.pieces().get_bb(Piece(PieceKind::Pawn, position.to_move()));

        if pawns.is_empty() {
            return;
        }

        let (up, up_left, up_right, fourth_rank, last_rank) = match position.to_move() {
            Color::White => (Direction::North, Direction::NorthWest, Direction::NorthEast, Bitboard::RANKS[3], Bitboard::RANKS[7]),
            Color::Black => (Direction::South, Direction::SouthEast, Direction::SouthWest, Bitboard::RANKS[4], Bitboard::RANKS[0]),
        };

        fn add_regulars(bb: Bitboard, dir: Direction, moves: &mut MoveList) {
            for sq in bb {
                moves.push(Move::Regular(sq << dir, sq));
            }
        }

        fn add_promos(bb: Bitboard, dir: Direction, moves: &mut MoveList) {
            for sq in bb {
                let orig = sq << dir;
                moves.push(Move::Promotion(orig, sq, PieceKind::Queen));
                moves.push(Move::Promotion(orig, sq, PieceKind::Rook));
                moves.push(Move::Promotion(orig, sq, PieceKind::Bishop));
                moves.push(Move::Promotion(orig, sq, PieceKind::Knight));
            }
        }

        // Forward
        let not_occ = !position.pieces().get_occ();
        let fwd = (pawns >> up) & not_occ;
        let fwd_no_promo = fwd - last_rank;
        let fwd_promo = fwd & last_rank;
        let fwd2 = (fwd >> up) & fourth_rank & not_occ;

        add_regulars(fwd_no_promo, up, moves);
        add_promos(fwd_promo, up, moves);
        fwd2.into_iter()
            .for_each(|sq| moves.push(Move::Regular(sq << up << up, sq)));

        // Attacks
        let opponent_occ = position.pieces().get_occ_for(!position.to_move());
        let left = pawns >> up_left;
        let right = pawns >> up_right;
        let left_atk = left & opponent_occ;
        let right_atk = right & opponent_occ;
        let left_atk_no_promo = left_atk - last_rank;
        let right_atk_no_promo = right_atk - last_rank;
        let left_atk_promo = left_atk & last_rank;
        let right_atk_promo = right_atk & last_rank;

        add_regulars(left_atk_no_promo, up_left, moves);
        add_regulars(right_atk_no_promo, up_right, moves);
        add_promos(left_atk_promo, up_left, moves);
        add_promos(right_atk_promo, up_right, moves);

        // En passant
        if let Some(sq) = position.en_passant_sq() {
            let ep_square_bb = bb!(sq);

            if !((left & ep_square_bb).is_empty()) {
                moves.push(Move::EnPassant(sq << up_left, sq));
            }
            if !((right & ep_square_bb).is_empty()) {
                moves.push(Move::EnPassant(sq << up_right, sq));
            }
        }
    }

    fn gen_bishop_attacks(&self, position: &Position, sq: Square) -> Bitboard {
        let occ: u64 = position.pieces().get_occ().into();
        let key = occ.pext(self.bishop_masks[sq].into()) as usize;
        let offset = self.bishop_offsets[sq];
        unsafe {
            *self.slider_attacks.get_unchecked(offset + key)
        }
    }

    fn gen_rook_attacks(&self, position: &Position, sq: Square) -> Bitboard {
        let occ: u64 = position.pieces().get_occ().into();
        let key = occ.pext(self.rook_masks[sq].into()) as usize;
        let offset = self.rook_offsets[sq];
        unsafe {
            *self.slider_attacks.get_unchecked(offset + key)
        }
    }

    fn gen_attacks_from_sq(&self, position: &Position, pce: Piece, sq: Square) -> Bitboard {
        match pce.kind() {
            PieceKind::Pawn => match pce.color() {
                Color::White => self.white_pawn_attacks[sq],
                Color::Black => self.black_pawn_attacks[sq],
            },
            PieceKind::Knight => self.knight_attacks[sq],
            PieceKind::Bishop => self.gen_bishop_attacks(position, sq),
            PieceKind::Rook => self.gen_rook_attacks(position, sq),
            PieceKind::Queen => self.gen_bishop_attacks(position, sq) | self.gen_rook_attacks(position, sq),
            PieceKind::King => self.king_attacks[sq],
        }
    }

    fn gen_attacks(&self, position: &Position, pce: Piece) -> Bitboard {
        match pce.kind() {
            PieceKind::Pawn => {
                let pawns = position.pieces().get_bb(pce);
                let (left, right) = match pce.color() {
                    Color::White => (Direction::NorthWest, Direction::NorthEast),
                    Color::Black => (Direction::SouthEast, Direction::SouthWest),
                };
                (pawns >> left) | (pawns >> right)
            },
            _ => {
                let pieces = position.pieces().get_bb(pce);
                pieces.into_iter()
                    .fold(Bitboard::new(), |atks, sq| unsafe {
                        atks | self.gen_attacks_from_sq(position, pce, sq)
                    })
            }
        }
    }

    fn gen_danger_sqs(&mut self, position: &Position) {
        let opponent = !position.to_move();

        self.danger_sqs = self.gen_attacks(position, Piece(PieceKind::Pawn, opponent))
            | self.gen_attacks(position, Piece(PieceKind::Knight, opponent))
            | self.gen_attacks(position, Piece(PieceKind::Bishop, opponent))
            | self.gen_attacks(position, Piece(PieceKind::Rook, opponent))
            | self.gen_attacks(position, Piece(PieceKind::Queen, opponent))
            | self.gen_attacks(position, Piece(PieceKind::King, opponent));
    }

    /// Generates all non-pawn moves for the given `PieceKind` `kind`, except castling moves
    fn gen_non_pawn_moves(&self, position: &Position, kind: PieceKind, moves: &mut MoveList) {
        let pce = Piece(kind, position.to_move());
        let pieces = position.pieces().get_bb(pce);

        let own_occ = position.pieces().get_occ_for(position.to_move());

        let danger_sqs;
        if pce.kind() == PieceKind::King {
            danger_sqs = self.danger_sqs;
        } else {
            danger_sqs = Bitboard::new();
        }

        for from in pieces {
            let legal = self.gen_attacks_from_sq(position, pce, from) - own_occ - danger_sqs;

            for to in legal {
                moves.push(Move::Regular(from, to));
            }
        }
    }

    fn gen_castling_moves(&self, position: &Position, moves: &mut MoveList) {
        fn gen_castling_move(position: &Position, side: Side, danger_sqs: Bitboard, moves: &mut MoveList) {
            if !position.castling().get(position.to_move(), side) {
                return;
            }

            use Square::*;
            let (castling_sqs, no_occ_sqs) = match (position.to_move(), side) {
                (Color::White, Side::KingSide) => (bb!(E1, F1, G1), bb!(F1, G1)),
                (Color::Black, Side::KingSide) => (bb!(E8, F8, G8), bb!(F8, G8)),
                (Color::White, Side::QueenSide) => (bb!(E1, D1, C1), bb!(D1, C1, B1)),
                (Color::Black, Side::QueenSide) => (bb!(E8, D8, C8), bb!(D8, C8, B8)),
            };

            let occ = position.pieces().get_occ();

            if ((castling_sqs & danger_sqs) | (no_occ_sqs & occ)).is_empty() {
                moves.push(Move::Castling(side));
            }
        }

        gen_castling_move(position, Side::KingSide, self.danger_sqs, moves);
        gen_castling_move(position, Side::QueenSide, self.danger_sqs, moves);
    }

    fn checkers(&self, position: &Position) -> Bitboard {
        let king_sq = position.pieces().get_king_sq(position.to_move());

        let pawn_pce = Piece(PieceKind::Pawn, !position.to_move());
        let opp_pawns = position.pieces().get_bb(pawn_pce);
        let our_pawn_pce = Piece(PieceKind::Pawn, position.to_move());
        let pawn_checkers = self.gen_attacks_from_sq(position, our_pawn_pce, king_sq) & opp_pawns;

        let knight_pce = Piece(PieceKind::Knight, !position.to_move());
        let opp_knights = position.pieces().get_bb(knight_pce);
        let knight_checkers = self.gen_attacks_from_sq(position, knight_pce, king_sq) & opp_knights;

        let bishop_pce = Piece(PieceKind::Bishop, !position.to_move());
        let opp_bishops = position.pieces().get_bb(bishop_pce);
        let bishop_checkers = self.gen_attacks_from_sq(position, bishop_pce, king_sq) & opp_bishops;

        let rook_pce = Piece(PieceKind::Rook, !position.to_move());
        let opp_rooks = position.pieces().get_bb(rook_pce);
        let rook_checkers = self.gen_attacks_from_sq(position, rook_pce, king_sq) & opp_rooks;

        let queen_pce = Piece(PieceKind::Queen, !position.to_move());
        let opp_queens = position.pieces().get_bb(queen_pce);
        let queen_checkers = self.gen_attacks_from_sq(position, queen_pce, king_sq) & opp_queens;

        pawn_checkers | knight_checkers | bishop_checkers | rook_checkers | queen_checkers
    }

    pub fn gen_all_moves(&mut self, position: &Position) -> MoveList {
        let mut moves = MoveList::new();

        self.gen_danger_sqs(position);

        self.gen_pawn_moves(position, &mut moves);
        self.gen_non_pawn_moves(position, PieceKind::Bishop, &mut moves);
        self.gen_non_pawn_moves(position, PieceKind::Rook, &mut moves);
        self.gen_non_pawn_moves(position, PieceKind::Queen, &mut moves);
        self.gen_non_pawn_moves(position, PieceKind::King, &mut moves);

        moves
    }
}