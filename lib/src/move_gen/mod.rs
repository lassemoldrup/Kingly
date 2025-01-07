use crate::collections::MoveList;
use crate::position::Position;
use crate::tables::Tables;
use crate::types::{
    Bitboard, BoardVector, Color, File, Move, Piece, PieceKind, Rank, Side, Square,
};
use crate::{bb, mv};

#[cfg(test)]
mod tests;

/// A move generator.
#[derive(Clone, Copy)]
pub struct MoveGen {
    tables: &'static Tables,
}

impl MoveGen {
    /// Initializes a new `MoveGen` instance by initializing the [`Tables`].
    /// This can therefore be an expensive operation.
    pub fn init() -> Self {
        Self::from_tables(Tables::get_or_init())
    }

    /// Creates a new `MoveGen` instance from the given [`Tables`].
    pub fn from_tables(tables: &'static Tables) -> Self {
        Self { tables }
    }

    fn gen_moves_and_check<const ONLY_CAPTURES: bool>(
        &self,
        position: &Position,
    ) -> (MoveList, bool) {
        use PieceKind::*;

        let mut state = MoveGenState::new(position, self.tables);
        state.set_danger_sqs();

        let check = state.danger_sqs.contains(state.king_sq);
        if check {
            let checkers = state.checkers();

            if checkers.len() == 2 {
                state.gen_non_pawn_moves::<ONLY_CAPTURES>(King, !bb!());
            } else {
                state.set_pin_rays();
                let checking_sq = checkers.into_iter().next().unwrap();
                // Can't block a check with a capture, unless capturing the checker
                let blocking_sqs = if ONLY_CAPTURES {
                    checkers
                } else {
                    self.tables.ray_to[state.king_sq][checking_sq] | checkers
                };

                state.gen_pawn_moves::<ONLY_CAPTURES>(blocking_sqs);
                for kind in [Knight, Bishop, Rook, Queen] {
                    state.gen_non_pawn_moves::<ONLY_CAPTURES>(kind, blocking_sqs);
                }
                state.gen_non_pawn_moves::<ONLY_CAPTURES>(King, !bb!());
            }
        } else {
            state.set_pin_rays();
            state.gen_pawn_moves::<ONLY_CAPTURES>(!bb!());
            for kind in [Knight, Bishop, Rook, Queen] {
                state.gen_non_pawn_moves::<ONLY_CAPTURES>(kind, !bb!());
            }
            state.gen_non_pawn_moves::<ONLY_CAPTURES>(King, !bb!());
            if !ONLY_CAPTURES {
                state.gen_castling_moves();
            }
        }

        (state.moves, check)
    }

    pub fn gen_all_moves(&self, position: &Position) -> MoveList {
        self.gen_moves_and_check::<false>(position).0
    }

    pub fn gen_all_moves_and_check(&self, position: &Position) -> (MoveList, bool) {
        self.gen_moves_and_check::<false>(position)
    }

    pub fn gen_captures(&self, position: &Position) -> MoveList {
        self.gen_moves_and_check::<true>(position).0
    }

    pub fn is_check(&self, position: &Position) -> bool {
        let mut state = MoveGenState::new(position, self.tables);
        state.set_danger_sqs();
        state.danger_sqs.contains(state.king_sq)
    }

    pub fn perft(&self, mut position: Position, depth: i8) -> u64 {
        if depth == 0 {
            return 1;
        }

        fn inner(move_gen: &MoveGen, position: &mut Position, depth: i8) -> u64 {
            let moves = move_gen.gen_all_moves(position);
            if depth == 1 {
                return moves.len() as u64;
            }

            let mut count = 0;
            for mv in moves {
                position.make_move(mv);
                count += inner(move_gen, position, depth - 1);
                position.unmake_move();
            }

            count
        }

        inner(self, &mut position, depth)
    }
}

struct MoveGenState<'p> {
    moves: MoveList,
    king_sq: Square,
    occupied: Bitboard,
    pin_rays: Bitboard,
    danger_sqs: Bitboard,
    position: &'p Position,
    tables: &'static Tables,
}

impl<'p> MoveGenState<'p> {
    fn new(position: &'p Position, tables: &'static Tables) -> Self {
        Self {
            moves: MoveList::new(),
            king_sq: position.pieces.king_sq_for(position.to_move),
            occupied: position.pieces.occupied(),
            pin_rays: bb!(),
            danger_sqs: bb!(),
            position,
            tables,
        }
    }

    /// Generates all non-pawn moves for the given `PieceKind` `kind`, except
    /// castling moves
    fn gen_non_pawn_moves<const ONLY_CAPTURES: bool>(
        &mut self,
        kind: PieceKind,
        blocking_sqs: Bitboard,
    ) {
        let pce = Piece(kind, self.position.to_move);
        let pieces = self.position.pieces.get_bb(pce);

        for from in pieces {
            // TODO: Check if this king check gets compiled away
            let pin_ray = if pce.kind() != PieceKind::King && self.pin_rays.contains(from) {
                self.pin_rays & self.tables.line_through[from][self.king_sq]
            } else {
                !bb!()
            };

            let opp_occ = self.position.pieces.occupied_for(!self.position.to_move);
            let mut legal_atks =
                self.tables.gen_attacks_from_sq(self.occupied, pce, from) & pin_ray & blocking_sqs;
            if pce.kind() == PieceKind::King {
                legal_atks -= self.danger_sqs;
            }

            for to in legal_atks & opp_occ {
                self.moves.push(mv!(from x to));
            }

            if !ONLY_CAPTURES {
                let own_occ = self.position.pieces.occupied_for(self.position.to_move);
                for to in legal_atks & !own_occ & !opp_occ {
                    self.moves.push(mv!(from -> to));
                }
            }
        }
    }

    fn gen_pawn_moves<const ONLY_CAPTURES: bool>(&mut self, blocking_sqs: Bitboard) {
        let pawns = self
            .position
            .pieces
            .get_bb(Piece(PieceKind::Pawn, self.position.to_move));

        if pawns.is_empty() {
            return;
        }

        let (up, up_left, up_right, fourth_rank, last_rank, left_file, right_file) =
            match self.position.to_move {
                Color::White => (
                    BoardVector::NORTH,
                    BoardVector::NORTH_WEST,
                    BoardVector::NORTH_EAST,
                    Bitboard::from(Rank::Fourth),
                    Bitboard::from(Rank::Eighth),
                    Bitboard::from(File::A),
                    Bitboard::from(File::H),
                ),
                Color::Black => (
                    BoardVector::SOUTH,
                    BoardVector::SOUTH_EAST,
                    BoardVector::SOUTH_WEST,
                    Bitboard::from(Rank::Fifth),
                    Bitboard::from(Rank::First),
                    Bitboard::from(File::H),
                    Bitboard::from(File::A),
                ),
            };

        macro_rules! add_moves {
            ( $bb:expr => $dir:expr, |$from:ident, $to:ident| $block:block ) => {
                for to in $bb {
                    let from = to - $dir;
                    if self.is_prohibited_by_pin(from, to) {
                        continue;
                    }
                    let $from = from;
                    let $to = to;
                    $block
                }
            };
        }

        macro_rules! add_regulars {
            ( $bb:expr => $dir:expr, $capture:expr ) => {
                add_moves!($bb => $dir, |from, to| {
                    self.moves.push(Move::new_regular(from, to, $capture));
                });
            };
        }

        macro_rules! add_promos {
            ( $bb:expr => $dir:expr, $capture:expr ) => {
                add_moves!($bb => $dir, |from, to| {
                    self.moves.push(Move::new_promotion(from, to, PieceKind::Queen, $capture));
                    self.moves.push(Move::new_promotion(from, to, PieceKind::Knight, $capture));
                    self.moves.push(Move::new_promotion(from, to, PieceKind::Rook, $capture));
                    self.moves.push(Move::new_promotion(from, to, PieceKind::Bishop, $capture));
                });
            };
        }

        macro_rules! add_en_passant {
            ( $to:expr => $dir:expr ) => {
                let from = $to - $dir;
                let ep_pawn_sq = $to - up;

                let fifth_rank = Bitboard::RANKS[from.rank() as usize];
                let opp_rooks = self.position
                    .pieces
                    .get_bb(Piece(PieceKind::Rook, !self.position.to_move));
                let opp_queens = self.position
                    .pieces
                    .get_bb(Piece(PieceKind::Queen, !self.position.to_move));
                let occ = self.position.pieces.occupied() - bb!(from, ep_pawn_sq);
                let ep_pinners =
                    self.tables.gen_rook_attacks(occ, self.king_sq) & fifth_rank & (opp_rooks | opp_queens);

                if self.is_prohibited_by_pin(from, $to) || !ep_pinners.is_empty()
                {
                    return;
                }

                self.moves.push(mv!(from ep $to));
            };
        }

        // Forward
        if !ONLY_CAPTURES {
            let fwd = (pawns >> up) - self.occupied;
            let legal_fwd = fwd & blocking_sqs;
            let fwd_no_promo = legal_fwd - last_rank;
            let fwd_promo = legal_fwd & last_rank;
            let fwd2 = ((fwd >> up) & fourth_rank & blocking_sqs) - self.occupied;

            add_regulars!(fwd_no_promo => up, false);
            add_promos!(fwd_promo => up, false);
            add_moves!(fwd2 => 2 * up, |from, to| {
                self.moves.push(mv!(from -> to));
            });
        }

        // Attacks
        let legal_atk = self.position.pieces.occupied_for(!self.position.to_move) & blocking_sqs;
        let left = (pawns >> up_left) - right_file;
        let right = (pawns >> up_right) - left_file;
        let left_atk = left & legal_atk;
        let right_atk = right & legal_atk;
        let left_atk_no_promo = left_atk - last_rank;
        let right_atk_no_promo = right_atk - last_rank;
        let left_atk_promo = left_atk & last_rank;
        let right_atk_promo = right_atk & last_rank;

        add_regulars!(left_atk_no_promo => up_left, true);
        add_regulars!(right_atk_no_promo => up_right, true);
        add_promos!(left_atk_promo => up_left, true);
        add_promos!(right_atk_promo => up_right, true);

        // En passant
        if let Some(sq) = self.position.en_passant_sq {
            let ep_square_bb = bb!(sq);
            let ep_pawn_sq = sq - up;

            // Note: It will never be possible to block a check with en passant as the
            // opponent's last move will have to have been a pawn move,
            // which doesn't allow for a blockable discovered check
            if blocking_sqs.contains(ep_pawn_sq) {
                if !((left & ep_square_bb).is_empty()) {
                    add_en_passant!(sq => up_left);
                }
                if !((right & ep_square_bb).is_empty()) {
                    add_en_passant!(sq => up_right);
                }
            }
        }
    }

    fn is_prohibited_by_pin(&self, from: Square, to: Square) -> bool {
        if self.pin_rays.contains(from) {
            let pin_ray = self.pin_rays & self.tables.line_through[from][self.king_sq];
            !pin_ray.contains(to)
        } else {
            false
        }
    }

    fn gen_castling_moves(&mut self) {
        let mut gen_castling_move = |side| {
            if !self.position.castling.get(self.position.to_move, side) {
                return;
            }

            let (castling_sqs, no_occ_sqs) = match (self.position.to_move, side) {
                (Color::White, Side::KingSide) => (bb!(E1, F1, G1), bb!(F1, G1)),
                (Color::Black, Side::KingSide) => (bb!(E8, F8, G8), bb!(F8, G8)),
                (Color::White, Side::QueenSide) => (bb!(E1, D1, C1), bb!(D1, C1, B1)),
                (Color::Black, Side::QueenSide) => (bb!(E8, D8, C8), bb!(D8, C8, B8)),
            };

            if ((castling_sqs & self.danger_sqs) | (no_occ_sqs & self.occupied)).is_empty() {
                let king_sq = Square::king_starting(self.position.to_move);
                let castling_sq = Square::king_castling_dest(self.position.to_move, side);
                self.moves.push(Move::new_castling(king_sq, castling_sq));
            }
        };

        gen_castling_move(Side::KingSide);
        gen_castling_move(Side::QueenSide);
    }

    fn checkers(&self) -> Bitboard {
        let pawn_pce = Piece(PieceKind::Pawn, !self.position.to_move);
        let opp_pawns = self.position.pieces.get_bb(pawn_pce);
        let our_pawn_pce = Piece(PieceKind::Pawn, self.position.to_move);
        let pawn_checkers =
            self.tables
                .gen_attacks_from_sq(self.occupied, our_pawn_pce, self.king_sq)
                & opp_pawns;

        let knight_pce = Piece(PieceKind::Knight, !self.position.to_move);
        let opp_knights = self.position.pieces.get_bb(knight_pce);
        let knight_checkers =
            self.tables
                .gen_attacks_from_sq(self.occupied, knight_pce, self.king_sq)
                & opp_knights;

        let queen_pce = Piece(PieceKind::Queen, !self.position.to_move);
        let opp_queens = self.position.pieces.get_bb(queen_pce);

        let bishop_pce = Piece(PieceKind::Bishop, !self.position.to_move);
        let opp_bishops = self.position.pieces.get_bb(bishop_pce);
        let bishop_queen_checkers =
            self.tables
                .gen_attacks_from_sq(self.occupied, bishop_pce, self.king_sq)
                & (opp_bishops | opp_queens);

        let rook_pce = Piece(PieceKind::Rook, !self.position.to_move);
        let opp_rooks = self.position.pieces.get_bb(rook_pce);
        let rook_queen_checkers =
            self.tables
                .gen_attacks_from_sq(self.occupied, rook_pce, self.king_sq)
                & (opp_rooks | opp_queens);

        pawn_checkers | knight_checkers | bishop_queen_checkers | rook_queen_checkers
    }

    fn set_pin_rays(&mut self) {
        let bishop_pce = Piece(PieceKind::Bishop, !self.position.to_move);
        let rook_pce = Piece(PieceKind::Rook, !self.position.to_move);
        let queen_pce = Piece(PieceKind::Queen, !self.position.to_move);

        let opp_bishops = self.position.pieces.get_bb(bishop_pce);
        let opp_rooks = self.position.pieces.get_bb(rook_pce);
        let opp_queens = self.position.pieces.get_bb(queen_pce);
        let opp_occ = self.position.pieces.occupied_for(!self.position.to_move);

        let bishop_queen_pinners =
            self.tables.gen_bishop_attacks(opp_occ, self.king_sq) & (opp_bishops | opp_queens);
        let rook_queen_pinners =
            self.tables.gen_rook_attacks(opp_occ, self.king_sq) & (opp_rooks | opp_queens);
        let pinners = bishop_queen_pinners | rook_queen_pinners;

        let mut pin_rays = bb!();

        // No need to only look at our own pieces (which is slower), since opp pieces
        // are removed above
        for sq in pinners {
            let pin_ray = self.tables.ray_to[self.king_sq][sq];
            let potential_pin = (pin_ray - pinners) & self.occupied;
            if potential_pin.len() == 1 {
                pin_rays |= pin_ray;
            }
        }
        self.pin_rays = pin_rays;
    }

    fn set_danger_sqs(&mut self) {
        let opponent = !self.position.to_move;
        let pawn_pce = Piece(PieceKind::Pawn, opponent);
        let knight_pce = Piece(PieceKind::Knight, opponent);
        let bishop_pce = Piece(PieceKind::Bishop, opponent);
        let rook_pce = Piece(PieceKind::Rook, opponent);
        let queen_pce = Piece(PieceKind::Queen, opponent);
        let king_pce = Piece(PieceKind::King, opponent);
        // Avoid allowing the king to step backwards away from a checking sliding piece
        let occ = self.occupied
            - self
                .position
                .pieces
                .get_bb(Piece(PieceKind::King, self.position.to_move));

        let tbl = self.tables;
        self.danger_sqs = tbl.gen_attacks(self.position.pieces.get_bb(pawn_pce), occ, pawn_pce)
            | tbl.gen_attacks(self.position.pieces.get_bb(knight_pce), occ, knight_pce)
            | tbl.gen_attacks(self.position.pieces.get_bb(bishop_pce), occ, bishop_pce)
            | tbl.gen_attacks(self.position.pieces.get_bb(rook_pce), occ, rook_pce)
            | tbl.gen_attacks(self.position.pieces.get_bb(queen_pce), occ, queen_pce)
            | tbl.gen_attacks(self.position.pieces.get_bb(king_pce), occ, king_pce);
    }
}
