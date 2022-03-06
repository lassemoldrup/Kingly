use bitintr::Pext;

use crate::bb;
use crate::framework::{Position as _, Side};
use crate::framework::color::Color;
use crate::framework::direction::Direction;
use crate::framework::moves::{Move, MoveList};
use crate::framework::piece::{Piece, PieceKind};
use crate::framework::square::Square;
use crate::framework::util::{get_castling_sq, get_king_sq};
use crate::standard::bitboard::Bitboard;
use crate::standard::tables::Tables;

use super::Position;

#[cfg(test)]
mod tests;

pub struct MoveGen {
    tables: &'static Tables,
}

impl MoveGen {
    pub fn new(tables: &'static Tables) -> Self {
        Self {
            tables,
        }
    }

    fn is_prohibited_by_pin(&self, from: Square, to: Square, king_sq: Square, pin_rays: Bitboard) -> bool {
        if pin_rays.contains(from) {
            let pin_ray = pin_rays & self.tables.line_through[from][king_sq];
            !pin_ray.contains(to)
        } else {
            false
        }
    }

    fn gen_pawn_moves<const CAPTURES: bool>(&self, position: &Position, blocking_sqs: Bitboard, pin_rays: Bitboard, moves: &mut MoveList) {
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
                let from = unsafe {
                    to.shift(-dir)
                };

                if move_gen.is_prohibited_by_pin(from, to, king_sq, pin_rays) {
                    continue;
                }

                moves.push(Move::new_regular(from, to));
            }
        }

        fn add_promos(move_gen: &MoveGen, bb: Bitboard, dir: Direction, pin_rays: Bitboard, king_sq: Square, moves: &mut MoveList) {
            for to in bb {
                let from = unsafe {
                    to.shift(-dir)
                };

                if move_gen.is_prohibited_by_pin(from, to, king_sq, pin_rays) {
                    continue;
                }

                moves.push(Move::new_promotion(from, to, PieceKind::Queen));
                moves.push(Move::new_promotion(from, to, PieceKind::Knight));
                moves.push(Move::new_promotion(from, to, PieceKind::Rook));
                moves.push(Move::new_promotion(from, to, PieceKind::Bishop));
            }
        }

        let king_sq = position.pieces().get_king_sq(position.to_move());

        // Forward
        if !CAPTURES {
            let occ = position.pieces().get_occ();
            let fwd = (pawns >> up) - occ;
            let legal_fwd = fwd & blocking_sqs;
            let fwd_no_promo = legal_fwd - last_rank;
            let fwd_promo = legal_fwd & last_rank;
            let fwd2 = ((fwd >> up) & fourth_rank & blocking_sqs) - occ;

            add_regulars(self, fwd_no_promo, up, pin_rays, king_sq, moves);
            add_promos(self, fwd_promo, up, pin_rays, king_sq, moves);
            for to in fwd2 {
                let from = unsafe {
                    to.shift(-up).shift(-up)
                };

                if self.is_prohibited_by_pin(from, to, king_sq, pin_rays) {
                    continue;
                }

                moves.push(Move::new_regular(from, to));
            }
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
        fn add_en_passant(move_gen: &MoveGen, position: &Position, dir: Direction, to: Square,
                          ep_pawn_sq: Square, pin_rays: Bitboard, king_sq: Square, moves: &mut MoveList) {
            let from = unsafe {
                to.shift(-dir)
            };

            let fifth_rank = Bitboard::RANKS[from.rank() as usize];
            let opp_rooks = position.pieces().get_bb(Piece(PieceKind::Rook, !position.to_move()));
            let opp_queens = position.pieces().get_bb(Piece(PieceKind::Queen, !position.to_move()));
            let occ = position.pieces().get_occ() - bb!(from, ep_pawn_sq);
            let ep_pinners = move_gen.gen_rook_attacks(occ, king_sq) & fifth_rank & (opp_rooks | opp_queens);

            if move_gen.is_prohibited_by_pin(from, to, king_sq, pin_rays) || !ep_pinners.is_empty() {
                return;
            }

            moves.push(Move::new_en_passant(from, to));
        }

        if let Some(sq) = position.en_passant_sq() {
            let ep_square_bb = bb!(sq);

            let ep_pawn_sq = unsafe {
                sq.shift(-up)
            };

            // Note: It will never be possible to block a check with en passant as the opponent's
            //       last move will have to have been a pawn move, which doesn't allow for a
            //       blockable discovered check
            if blocking_sqs.contains(ep_pawn_sq) {
                if !((left & ep_square_bb).is_empty()) {
                    add_en_passant(self, position, up_left, sq, ep_pawn_sq, pin_rays, king_sq, moves);
                }
                if !((right & ep_square_bb).is_empty()) {
                    add_en_passant(self, position, up_right, sq, ep_pawn_sq, pin_rays, king_sq, moves);
                }
            }

        }
    }

    fn gen_bishop_attacks(&self, occ: Bitboard, sq: Square) -> Bitboard {
        let occ: u64 = occ.into();
        let key = occ.pext(self.tables.bishop_masks[sq].into()) as usize;
        let offset = self.tables.bishop_offsets[sq];
        unsafe {
            // TODO: Save the offset as a pointer instead?
            *self.tables.slider_attacks.get_unchecked(offset + key)
        }
    }

    fn gen_rook_attacks(&self, occ: Bitboard, sq: Square) -> Bitboard {
        let occ: u64 = occ.into();
        let key = occ.pext(self.tables.rook_masks[sq].into()) as usize;
        let offset = self.tables.rook_offsets[sq];
        unsafe {
            *self.tables.slider_attacks.get_unchecked(offset + key)
        }
    }

    fn gen_attacks_from_sq(&self, occ: Bitboard, pce: Piece, sq: Square) -> Bitboard {
        match pce.kind() {
            PieceKind::Pawn => match pce.color() {
                Color::White => self.tables.white_pawn_attacks[sq],
                Color::Black => self.tables.black_pawn_attacks[sq],
            },
            PieceKind::Knight => self.tables.knight_attacks[sq],
            PieceKind::Bishop => self.gen_bishop_attacks(occ, sq),
            PieceKind::Rook => self.gen_rook_attacks(occ, sq),
            PieceKind::Queen =>
                self.gen_bishop_attacks(occ, sq)
                | self.gen_rook_attacks(occ, sq),
            PieceKind::King => self.tables.king_attacks[sq],
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
                .fold(bb!(), |atks, sq| atks | self.gen_attacks_from_sq(occ, pce, sq))
        }
    }

    fn gen_danger_sqs(&self, position: &Position) -> Bitboard {
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

        self.gen_attacks(position.pieces().get_bb(pawn_pce), occ, pawn_pce)
            | self.gen_attacks(position.pieces().get_bb(knight_pce), occ, knight_pce)
            | self.gen_attacks(position.pieces().get_bb(bishop_pce), occ, bishop_pce)
            | self.gen_attacks(position.pieces().get_bb(rook_pce), occ, rook_pce)
            | self.gen_attacks(position.pieces().get_bb(queen_pce), occ, queen_pce)
            | self.gen_attacks(position.pieces().get_bb(king_pce), occ, king_pce)
    }

    /// Generates all non-pawn moves for the given `PieceKind` `kind`, except castling moves
    fn gen_non_pawn_moves<const CAPTURES: bool>(&self, position: &Position, kind: PieceKind,
        blocking_sqs: Bitboard, pin_rays: Bitboard, danger_sqs: Bitboard, moves: &mut MoveList)
    {
        let pce = Piece(kind, position.to_move());
        let pieces = position.pieces().get_bb(pce);

        let occ = position.pieces().get_occ();
        let king_sq = position.pieces().get_king_sq(position.to_move());
        
        let mut legal_sqs = if CAPTURES {
            let opp_occ = position.pieces().get_occ_for(!position.to_move());
            opp_occ & blocking_sqs
        } else {
            let own_occ = position.pieces().get_occ_for(position.to_move());
            !own_occ & blocking_sqs
        };

        if pce.kind() == PieceKind::King {
            legal_sqs -= danger_sqs;
        }

        for from in pieces {
            // TODO: Check if this king check gets compiled away
            let pin_ray = if pce.kind() != PieceKind::King && pin_rays.contains(from) {
                pin_rays & self.tables.line_through[from][king_sq]
            } else {
                !bb!()
            };

            let legal_atks = self.gen_attacks_from_sq(occ, pce, from) & legal_sqs & pin_ray;

            for to in legal_atks {
                moves.push(Move::new_regular(from, to));
            }
        }
    }

    fn gen_castling_moves(&self, position: &Position, danger_sqs: Bitboard, moves: &mut MoveList) {
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
                let king_sq = get_king_sq(position.to_move());
                let castling_sq = get_castling_sq(position.to_move(), side);
                moves.push(Move::new_castling(king_sq, castling_sq));
            }
        }

        gen_castling_move(position, Side::KingSide, danger_sqs, moves);
        gen_castling_move(position, Side::QueenSide, danger_sqs, moves);
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

        let mut pin_rays = bb!();

        // No need to only look at our own pieces (which is slower), since opp pieces are removed above
        let occ = position.pieces().get_occ();
        for sq in pinners {
            let pin_ray = self.tables.ray_to[king_sq][sq];
            let potential_pin = (pin_ray - pinners) & occ;
            if potential_pin.len() == 1 {
                pin_rays |= pin_ray;
            }
        }

        pin_rays
    }

    fn gen_moves_and_check<const CAPTURES: bool>(&self, position: &Position) -> (MoveList, bool) {
        let mut moves = MoveList::new();

        // TODO: Is this generated thrice?
        let king_sq = position.pieces().get_king_sq(position.to_move());
        let pin_rays = self.pin_rays(position);
        let danger_sqs = self.gen_danger_sqs(position);

        let check = danger_sqs.contains(king_sq);
        if check {
            let checkers = self.checkers(position);

            if checkers.len() == 2 {
                self.gen_non_pawn_moves::<CAPTURES>(position, PieceKind::King, !bb!(), pin_rays, danger_sqs, &mut moves);
            } else {
                let checking_sq = unsafe {
                    checkers.first_sq_unchecked()
                };
                // Can't block a check with a capture, unless capturing the checker
                let blocking_sqs = if CAPTURES {
                    checkers
                } else {
                    self.tables.ray_to[king_sq][checking_sq] | checkers
                };

                self.gen_pawn_moves::<CAPTURES>(position, blocking_sqs, pin_rays, &mut moves);
                self.gen_non_pawn_moves::<CAPTURES>(position, PieceKind::Knight, blocking_sqs, pin_rays, bb!(), &mut moves);
                self.gen_non_pawn_moves::<CAPTURES>(position, PieceKind::Bishop, blocking_sqs, pin_rays, bb!(), &mut moves);
                self.gen_non_pawn_moves::<CAPTURES>(position, PieceKind::Rook, blocking_sqs, pin_rays, bb!(), &mut moves);
                self.gen_non_pawn_moves::<CAPTURES>(position, PieceKind::Queen, blocking_sqs, pin_rays, bb!(), &mut moves);
                self.gen_non_pawn_moves::<CAPTURES>(position, PieceKind::King, !bb!(), bb!(), danger_sqs, &mut moves);
            }
        } else {
            self.gen_pawn_moves::<CAPTURES>(position, !bb!(), pin_rays, &mut moves);
            self.gen_non_pawn_moves::<CAPTURES>(position, PieceKind::Knight, !bb!(), pin_rays, bb!(), &mut moves);
            self.gen_non_pawn_moves::<CAPTURES>(position, PieceKind::Bishop, !bb!(), pin_rays, bb!(), &mut moves);
            self.gen_non_pawn_moves::<CAPTURES>(position, PieceKind::Rook, !bb!(), pin_rays, bb!(), &mut moves);
            self.gen_non_pawn_moves::<CAPTURES>(position, PieceKind::Queen, !bb!(), pin_rays, bb!(), &mut moves);
            self.gen_non_pawn_moves::<CAPTURES>(position, PieceKind::King, !bb!(), bb!(), danger_sqs, &mut moves);
            if !CAPTURES {
                self.gen_castling_moves(position, danger_sqs, &mut moves);
            }
        }

        (moves, check)
    }

    // TODO: Move out of MoveGen?
    pub(in crate::standard) fn get_mobility(&self, position: &Position, color: Color) -> usize {
        use PieceKind::*;

        let occ = position.pieces().get_occ();
        [Pawn, Knight, Bishop, Rook, Queen, King].into_iter()
            .map(|kind| {
                let pce = Piece(kind, color);
                let pce_bb = position.pieces().get_bb(pce);
                self.gen_attacks(pce_bb, occ, pce).len()
            })
            .sum()
    }
}

impl crate::framework::MoveGen<Position> for MoveGen {
    fn create() -> Self {
        Self::new(Tables::get())
    }

    fn gen_all_moves(&self, position: &Position) -> MoveList {
        self.gen_moves_and_check::<false>(position).0
    }

    fn gen_all_moves_and_check(&self, position: &Position) -> (MoveList, bool) {
        self.gen_moves_and_check::<false>(position)
    }

    fn gen_captures(&self, position: &Position) -> MoveList {
        self.gen_moves_and_check::<true>(position).0
    }
}