use std::convert::TryFrom;
use std::iter::repeat;

use bitintr::{Pdep, Pext};
use take_until::TakeUntilExt;

use crate::bb;
use crate::framework::Side;
use crate::framework::color::Color;
use crate::framework::direction::Direction;
use crate::framework::moves::{Move, MoveList};
use crate::framework::piece::{Piece, PieceKind};
use crate::framework::square::Square;
use crate::framework::square_map::SquareMap;
use crate::framework::square_vec::SquareVec;
use crate::standard::bitboard::Bitboard;
use crate::standard::position::Position;

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
    line_through: SquareMap<SquareMap<Bitboard>>, // The entire line between two squares, empty bb if not on line
    ray_to: SquareMap<SquareMap<Bitboard>>, // All the squares between two squares, includes the
                                            // destination square, empty bb if not on a line
}

impl MoveGen {
    pub fn new() -> Self {
        let white_pawn_attacks = Self::init_white_pawn_attacks();
        let black_pawn_attacks = Self::init_black_pawn_attacks();
        let knight_attacks = Self::init_knight_attacks();
        let king_attacks = Self::init_king_attacks();
        let bishop_masks = Self::init_bishop_masks();
        let rook_masks = Self::init_rook_masks();
        let line_through = Self::init_line_through();
        let ray_to = Self::init_ray_to();

        let mut move_gen = MoveGen {
            danger_sqs: Bitboard::new(),
            white_pawn_attacks,
            black_pawn_attacks,
            knight_attacks,
            king_attacks,
            bishop_masks,
            rook_masks,
            slider_attacks: Vec::new(),
            bishop_offsets: SquareMap::new([0; 64]),
            rook_offsets: SquareMap::new([0; 64]),
            line_through,
            ray_to,
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
                *bb -= Bitboard::RANKS[0];
            }
            if sq.rank() != 7 {
                *bb -= Bitboard::RANKS[7];
            }
            if sq.file() != 0 {
                *bb -= Bitboard::FILES[0];
            }
            if sq.file() != 7 {
                *bb -= Bitboard::FILES[7];
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

    fn init_line_through() -> SquareMap<SquareMap<Bitboard>> {
        let mut table = SquareMap::new([SquareMap::new([Bitboard::new(); 64]); 64]);

        for from in Square::iter() {
            let from_rank = from.rank() as i8;
            let from_file = from.file() as i8;

            for to in Square::iter() {
                if from == to {
                    continue;
                }

                let to_rank = to.rank() as i8;
                let to_file = to.file() as i8;
                let delta_rank = to_rank - from_rank;
                let delta_file = to_file - from_file;

                if from_rank == to_rank {
                    table[from][to] = Bitboard::RANKS[from.rank() as usize];
                } else if from_file == to_file {
                    table[from][to] = Bitboard::FILES[from.file() as usize];
                } else if delta_rank == delta_file {
                    let diag_idx = (7 + from_rank - from_file) as usize;
                    table[from][to] = Bitboard::DIAGS[diag_idx];
                } else if delta_rank == -delta_file {
                    let anit_diag_idx = (from_rank + from_file) as usize;
                    table[from][to] = Bitboard::ANTI_DIAGS[anit_diag_idx];
                }
            }
        }

        table
    }

    fn init_ray_to() -> SquareMap<SquareMap<Bitboard>> {
        let mut table = SquareMap::new([SquareMap::new([Bitboard::new(); 64]); 64]);

        for from in Square::iter() {
            let from_rank = from.rank() as i8;
            let from_file = from.file() as i8;

            for to in Square::iter() {
                if from == to {
                    table[from][to] = bb!(to);
                    continue;
                }

                let to_rank = to.rank() as i8;
                let to_file = to.file() as i8;
                let delta_rank = to_rank - from_rank;
                let delta_file = to_file - from_file;

                if delta_rank.abs() != delta_file.abs() && delta_rank != 0 && delta_file != 0 {
                    continue;
                }

                let rank_step = delta_rank.signum();
                let file_step = delta_file.signum();

                let mut rank = from_rank + rank_step;
                let mut file = from_file + file_step;
                while rank != to_rank || file != to_file {
                    let sq = Square::try_from((8 * rank + file) as u8).unwrap();
                    table[from][to] = table[from][to].add_sq(sq);

                    rank += rank_step;
                    file += file_step;
                }
                table[from][to] = table[from][to].add_sq(to);
            }
        }

        table
    }

    fn is_prohibited_by_pin(&self, from: Square, to: Square, king_sq: Square, pin_rays: Bitboard) -> bool {
        if pin_rays.contains(from) {
            let pin_ray = pin_rays & self.line_through[from][king_sq];
            !pin_ray.contains(to)
        } else {
            false
        }
    }

    fn gen_pawn_moves(&self, position: &Position, blocking_sqs: Bitboard, pin_rays: Bitboard, moves: &mut MoveList) {
        let pawns = position.pieces().get_bb(Piece(PieceKind::Pawn, position.to_move()));

        if pawns.is_empty() {
            return;
        }

        let (up, up_left, up_right, fourth_rank, last_rank) = match position.to_move() {
            Color::White => (Direction::North, Direction::NorthWest, Direction::NorthEast, Bitboard::RANKS[3], Bitboard::RANKS[7]),
            Color::Black => (Direction::South, Direction::SouthEast, Direction::SouthWest, Bitboard::RANKS[4], Bitboard::RANKS[0]),
        };

        fn add_regulars(move_gen: &MoveGen, bb: Bitboard, dir: Direction, pin_rays: Bitboard, king_sq: Square, moves: &mut MoveList) {
            for to in bb {
                let from = to << dir;

                if move_gen.is_prohibited_by_pin(from, to, king_sq, pin_rays) {
                    continue;
                }

                moves.push(Move::Regular(from, to));
            }
        }

        fn add_promos(move_gen: &MoveGen, bb: Bitboard, dir: Direction, pin_rays: Bitboard, king_sq: Square, moves: &mut MoveList) {
            for to in bb {
                let from = to << dir;

                if move_gen.is_prohibited_by_pin(from, to, king_sq, pin_rays) {
                    continue;
                }

                moves.push(Move::Promotion(from, to, PieceKind::Queen));
                moves.push(Move::Promotion(from, to, PieceKind::Knight));
                moves.push(Move::Promotion(from, to, PieceKind::Rook));
                moves.push(Move::Promotion(from, to, PieceKind::Bishop));
            }
        }

        let king_sq = position.pieces().get_king_sq(position.to_move());

        // Forward
        let legal_fwd = !position.pieces().get_occ() & blocking_sqs;
        let fwd = (pawns >> up) & legal_fwd;
        let fwd_no_promo = fwd - last_rank;
        let fwd_promo = fwd & last_rank;
        let fwd2 = (fwd >> up) & fourth_rank & legal_fwd;

        add_regulars(self, fwd_no_promo, up, pin_rays, king_sq, moves);
        add_promos(self, fwd_promo, up, pin_rays, king_sq, moves);
        for to in fwd2 {
            let from = to << up << up;

            if self.is_prohibited_by_pin(from, to, king_sq, pin_rays) {
                continue;
            }

            moves.push(Move::Regular(from, to));
        }

        // Attacks
        let legal_atk = position.pieces().get_occ_for(!position.to_move()) & blocking_sqs;
        let left = pawns >> up_left;
        let right = pawns >> up_right;
        let left_atk = left & legal_atk;
        let right_atk = right & legal_atk;
        let left_atk_no_promo = left_atk - last_rank;
        let right_atk_no_promo = right_atk - last_rank;
        let left_atk_promo = left_atk & last_rank;
        let right_atk_promo = right_atk & last_rank;

        add_regulars(self, left_atk_no_promo, up_left, pin_rays, king_sq, moves);
        add_regulars(self, right_atk_no_promo, up_right, pin_rays, king_sq, moves);
        add_promos(self, left_atk_promo, up_left, pin_rays, king_sq, moves);
        add_promos(self, right_atk_promo, up_right, pin_rays, king_sq, moves);

        // En passant
        fn add_en_passant(move_gen: &MoveGen, position: &Position, from: Square, to: Square,
                          ep_pawn_sq: Square, pin_rays: Bitboard, king_sq: Square, moves: &mut MoveList) {
            let fifth_rank = Bitboard::RANKS[from.rank() as usize];
            let opp_rooks = position.pieces().get_bb(Piece(PieceKind::Rook, !position.to_move()));
            let opp_queens = position.pieces().get_bb(Piece(PieceKind::Queen, !position.to_move()));
            let occ = position.pieces().get_occ() - bb!(from, ep_pawn_sq);
            let ep_pinners = move_gen.gen_rook_attacks(occ, king_sq) & fifth_rank & (opp_rooks | opp_queens);

            if move_gen.is_prohibited_by_pin(from, to, king_sq, pin_rays) || !ep_pinners.is_empty() {
                return;
            }

            moves.push(Move::EnPassant(from, to));
        }

        if let Some(sq) = position.en_passant_sq() {
            let ep_square_bb = bb!(sq);

            let ep_pawn_sq = sq << up;

            // Note: It will never be possible to block a check with en passant as the opponent's
            //       last move will have to have been a pawn move, which doesn't allow for a
            //       blockable discovered check
            if blocking_sqs.contains(ep_pawn_sq) {
                if !((left & ep_square_bb).is_empty()) {
                    let from = sq << up_left;
                    add_en_passant(self, position, from, sq, ep_pawn_sq, pin_rays, king_sq, moves);
                }
                if !((right & ep_square_bb).is_empty()) {
                    let from = sq << up_right;
                    add_en_passant(self, position, from, sq, ep_pawn_sq, pin_rays, king_sq, moves);
                }
            }

        }
    }

    fn gen_bishop_attacks(&self, occ: Bitboard, sq: Square) -> Bitboard {
        let occ: u64 = occ.into();
        let key = occ.pext(self.bishop_masks[sq].into()) as usize;
        let offset = self.bishop_offsets[sq];
        unsafe {
            // TODO: Save an instruction by saving the offset as a pointer instead?
            *self.slider_attacks.get_unchecked(offset + key)
        }
    }

    fn gen_rook_attacks(&self, occ: Bitboard, sq: Square) -> Bitboard {
        let occ: u64 = occ.into();
        let key = occ.pext(self.rook_masks[sq].into()) as usize;
        let offset = self.rook_offsets[sq];
        unsafe {
            *self.slider_attacks.get_unchecked(offset + key)
        }
    }

    fn gen_attacks_from_sq(&self, occ: Bitboard, pce: Piece, sq: Square) -> Bitboard {
        match pce.kind() {
            PieceKind::Pawn => match pce.color() {
                Color::White => self.white_pawn_attacks[sq],
                Color::Black => self.black_pawn_attacks[sq],
            },
            PieceKind::Knight => self.knight_attacks[sq],
            PieceKind::Bishop => self.gen_bishop_attacks(occ, sq),
            PieceKind::Rook => self.gen_rook_attacks(occ, sq),
            PieceKind::Queen =>
                self.gen_bishop_attacks(occ, sq)
                | self.gen_rook_attacks(occ, sq),
            PieceKind::King => self.king_attacks[sq],
        }
    }

    fn gen_attacks(&self, pce_bb: Bitboard, occ: Bitboard, pce: Piece) -> Bitboard {
        match pce.kind() {
            PieceKind::Pawn => {
                let (left, right) = match pce.color() {
                    Color::White => (Direction::NorthWest, Direction::NorthEast),
                    Color::Black => (Direction::SouthEast, Direction::SouthWest),
                };
                (pce_bb >> left) | (pce_bb >> right)
            },
            _ => pce_bb.into_iter()
                .fold(Bitboard::new(), |atks, sq| atks | self.gen_attacks_from_sq(occ, pce, sq))
        }
    }

    fn gen_danger_sqs(&mut self, position: &Position) {
        let opponent = !position.to_move();
        let pawn_pce = Piece(PieceKind::Pawn, opponent);
        let knight_pce = Piece(PieceKind::Knight, opponent);
        let bishop_pce = Piece(PieceKind::Bishop, opponent);
        let rook_pce = Piece(PieceKind::Rook, opponent);
        let queen_pce = Piece(PieceKind::Queen, opponent);
        let king_pce = Piece(PieceKind::King, opponent);
        // Avoid allowing the king to step backwards away from a checking sliding piece
        let occ = position.pieces().get_occ()
            - position.pieces().get_bb(Piece(PieceKind::King, position.to_move()));

        self.danger_sqs = self.gen_attacks(position.pieces().get_bb(pawn_pce), occ, pawn_pce)
            | self.gen_attacks(position.pieces().get_bb(knight_pce), occ, knight_pce)
            | self.gen_attacks(position.pieces().get_bb(bishop_pce), occ, bishop_pce)
            | self.gen_attacks(position.pieces().get_bb(rook_pce), occ, rook_pce)
            | self.gen_attacks(position.pieces().get_bb(queen_pce), occ, queen_pce)
            | self.gen_attacks(position.pieces().get_bb(king_pce), occ, king_pce);
    }

    /// Generates all non-pawn moves for the given `PieceKind` `kind`, except castling moves
    fn gen_non_pawn_moves(&self, position: &Position, kind: PieceKind, blocking_sqs: Bitboard, pin_rays: Bitboard, moves: &mut MoveList) {
        let pce = Piece(kind, position.to_move());
        let pieces = position.pieces().get_bb(pce);

        let occ = position.pieces().get_occ();
        let king_sq = position.pieces().get_king_sq(position.to_move());
        let own_occ = position.pieces().get_occ_for(position.to_move());
        let mut legal_sqs = !own_occ & blocking_sqs;

        if pce.kind() == PieceKind::King {
            legal_sqs -= self.danger_sqs;
        }

        for from in pieces {
            if pce.kind() != PieceKind::King && pin_rays.contains(from) {
                let pin_ray = pin_rays & self.line_through[from][king_sq];
                legal_sqs &= pin_ray;
            }

            let legal_atks = self.gen_attacks_from_sq(occ, pce, from) & legal_sqs;

            for to in legal_atks {
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
        let occ = position.pieces().get_occ();

        let pawn_pce = Piece(PieceKind::Pawn, !position.to_move());
        let opp_pawns = position.pieces().get_bb(pawn_pce);
        let our_pawn_pce = Piece(PieceKind::Pawn, position.to_move());
        let pawn_checkers = self.gen_attacks_from_sq(occ, our_pawn_pce, king_sq) & opp_pawns;

        let knight_pce = Piece(PieceKind::Knight, !position.to_move());
        let opp_knights = position.pieces().get_bb(knight_pce);
        let knight_checkers = self.gen_attacks_from_sq(occ, knight_pce, king_sq) & opp_knights;

        let queen_pce = Piece(PieceKind::Queen, !position.to_move());
        let opp_queens = position.pieces().get_bb(queen_pce);

        let bishop_pce = Piece(PieceKind::Bishop, !position.to_move());
        let opp_bishops = position.pieces().get_bb(bishop_pce);
        let bishop_queen_checkers = self.gen_attacks_from_sq(occ, bishop_pce, king_sq)
            & (opp_bishops | opp_queens);

        let rook_pce = Piece(PieceKind::Rook, !position.to_move());
        let opp_rooks = position.pieces().get_bb(rook_pce);
        let rook_queen_checkers = self.gen_attacks_from_sq(occ, rook_pce, king_sq)
            & (opp_rooks | opp_queens);

        pawn_checkers | knight_checkers | bishop_queen_checkers | rook_queen_checkers
    }

    fn pin_rays(&self, position: &Position) -> Bitboard {
        let king_sq = position.pieces().get_king_sq(position.to_move());

        let bishop_pce = Piece(PieceKind::Bishop, !position.to_move());
        let rook_pce = Piece(PieceKind::Rook, !position.to_move());
        let queen_pce = Piece(PieceKind::Queen, !position.to_move());

        let opp_bishops = position.pieces().get_bb(bishop_pce);
        let opp_rooks = position.pieces().get_bb(rook_pce);
        let opp_queens = position.pieces().get_bb(queen_pce);
        let opp_occ = position.pieces().get_occ_for(!position.to_move());

        let bishop_queen_pinners = self.gen_bishop_attacks(opp_occ, king_sq)
            & (opp_bishops | opp_queens);
        let rook_queen_pinners = self.gen_rook_attacks(opp_occ, king_sq)
            & (opp_rooks | opp_queens);
        let pinners = bishop_queen_pinners | rook_queen_pinners;

        let mut pin_rays = Bitboard::new();

        // No need to only look at our own pieces (which is slower), since opp pieces are removed above
        let occ = position.pieces().get_occ();
        for sq in pinners {
            let pin_ray = self.ray_to[king_sq][sq];
            let potential_pin = (pin_ray - pinners) & occ;
            if potential_pin.len() == 1 {
                pin_rays |= pin_ray;
            }
        }

        pin_rays
    }

    pub fn gen_all_moves(&mut self, position: &Position) -> MoveList {
        let mut moves = MoveList::new();

        self.gen_danger_sqs(position);

        // TODO: Is this generated thrice?
        let king_sq = position.pieces().get_king_sq(position.to_move());
        let pin_rays = self.pin_rays(position);
        if self.danger_sqs.contains(king_sq) {
            let checkers = self.checkers(position);

            if checkers.len() == 2 {
                self.gen_non_pawn_moves(position, PieceKind::King, !Bitboard::new(), pin_rays, &mut moves);
            } else {
                let checking_sq = unsafe {
                    checkers.first_sq_unchecked()
                };
                let blocking_sqs = self.ray_to[king_sq][checking_sq];

                self.gen_pawn_moves(position, blocking_sqs, pin_rays, &mut moves);
                self.gen_non_pawn_moves(position, PieceKind::Knight, blocking_sqs, pin_rays, &mut moves);
                self.gen_non_pawn_moves(position, PieceKind::Bishop, blocking_sqs, pin_rays, &mut moves);
                self.gen_non_pawn_moves(position, PieceKind::Rook, blocking_sqs, pin_rays, &mut moves);
                self.gen_non_pawn_moves(position, PieceKind::Queen, blocking_sqs, pin_rays, &mut moves);
                self.gen_non_pawn_moves(position, PieceKind::King, !Bitboard::new(), pin_rays, &mut moves);
            }
        } else {
            self.gen_pawn_moves(position, !Bitboard::new(), pin_rays, &mut moves);
            self.gen_non_pawn_moves(position, PieceKind::Knight, !Bitboard::new(), pin_rays, &mut moves);
            self.gen_non_pawn_moves(position, PieceKind::Bishop, !Bitboard::new(), pin_rays, &mut moves);
            self.gen_non_pawn_moves(position, PieceKind::Rook, !Bitboard::new(), pin_rays, &mut moves);
            self.gen_non_pawn_moves(position, PieceKind::Queen, !Bitboard::new(), pin_rays, &mut moves);
            self.gen_non_pawn_moves(position, PieceKind::King, !Bitboard::new(), pin_rays, &mut moves);
            self.gen_castling_moves(position, &mut moves);
        }

        moves
    }
}