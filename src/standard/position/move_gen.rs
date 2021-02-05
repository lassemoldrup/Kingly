use crate::standard::position::StandardPosition;
use arrayvec::ArrayVec;
use crate::framework::moves::Move;
use crate::framework::color::Color;
use crate::framework::direction::Direction;
use crate::framework::piece::{PieceKind, Piece};
use crate::framework::square_map::SquareMap;
use crate::framework::square_vec::SquareVec;
use crate::standard::bitboard::Bitboard;
use crate::bb;
use crate::framework::square::Square;
use std::hint::unreachable_unchecked;
use crate::framework::{CastlingRights, Side};

#[cfg(test)]
mod tests;

pub struct MoveGen {
    knight_attacks: SquareMap<Bitboard>,
    king_attacks: SquareMap<Bitboard>,
}

impl MoveGen {
    pub fn new() -> Self {
        let knight_attacks = Self::init_knight_attacks();
        let king_attacks = Self::init_king_attacks();

        MoveGen {
            knight_attacks,
            king_attacks,
        }
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

    // Todo: Change to const generic array once it becomes stable
    fn init_step_attacks(move_vecs: Vec<SquareVec>) -> SquareMap<Bitboard> {
        let mut table = SquareMap::new([Bitboard::new(); 64]);

        for (sq, ss) in table.iter_mut() {
            *ss = move_vecs.iter().fold(Bitboard::new(), |acc, vec| {
                match sq + *vec {
                    Some(dest) => acc.add_sq(dest),
                    None => acc,
                }
            });
        }

        table
    }

    fn gen_pawn_moves(&self, position: &StandardPosition, moves: &mut ArrayVec<[Move; 256]>) {
        let pawns = position.pieces.get_sqs(Piece(PieceKind::Pawn, position.to_move));

        if pawns.is_empty() {
            return;
        }

        let (up, up_left, up_right, fourth_rank, last_rank) = match position.to_move {
            Color::White => (Direction::North, Direction::NorthWest, Direction::NorthEast, Bitboard::RANKS[3], Bitboard::RANKS[7]),
            Color::Black => (Direction::South, Direction::SouthEast, Direction::SouthWest, Bitboard::RANKS[4], Bitboard::RANKS[0]),
        };

        fn add_regulars(bb: Bitboard, dir: Direction, moves: &mut ArrayVec<[Move; 256]>) {
            for sq in bb {
                moves.push(Move::Regular(sq << dir, sq));
            }
        }

        fn add_promos(bb: Bitboard, dir: Direction, moves: &mut ArrayVec<[Move; 256]>) {
            for sq in bb {
                let orig = sq << dir;
                moves.push(Move::Promotion(orig, sq, PieceKind::Queen));
                moves.push(Move::Promotion(orig, sq, PieceKind::Rook));
                moves.push(Move::Promotion(orig, sq, PieceKind::Bishop));
                moves.push(Move::Promotion(orig, sq, PieceKind::Knight));
            }
        }

        // Forward
        let not_occ = !position.pieces.get_occupied();
        let fwd = (pawns >> up) & not_occ;
        let fwd_no_promo = fwd - last_rank;
        let fwd_promo = fwd & last_rank;
        let fwd2 = (fwd >> up) & fourth_rank & not_occ;

        add_regulars(fwd_no_promo, up, moves);
        add_promos(fwd_promo, up, moves);
        fwd2.into_iter()
            .for_each(|sq| moves.push(Move::Regular(sq << up << up, sq)));

        // Attacks
        let opponent_occ = position.pieces.get_sqs_for(!position.to_move);
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
        if let Some(sq) = position.en_passant_sq {
            let ep_square_bb = bb!(sq);

            if !((left & ep_square_bb).is_empty()) {
                moves.push(Move::EnPassant(sq << up_left, sq));
            }
            if !((right & ep_square_bb).is_empty()) {
                moves.push(Move::EnPassant(sq << up_right, sq));
            }
        }
    }

    unsafe fn gen_non_pawn_attacks_from_sq(&self, pce: Piece, sq: Square) -> Bitboard {
        match pce.kind() {
            PieceKind::Pawn => unreachable_unchecked(),
            PieceKind::Knight => self.knight_attacks[sq],
            PieceKind::Bishop => Bitboard::new(), // TODO
            PieceKind::Rook => Bitboard::new(), // TODO
            PieceKind::Queen => Bitboard::new(), // TODO
            PieceKind::King => self.king_attacks[sq],
        }
    }

    /// # Safety
    /// `kind` can't be `Pawn`
    unsafe fn gen_non_pawn_attacks(&self, position: &StandardPosition, pce: Piece) -> Bitboard {
        let pieces = position.pieces.get_sqs(pce);
        pieces.into_iter()
            .fold(Bitboard::new(), |atks, sq|
                atks | self.gen_non_pawn_attacks_from_sq(pce, sq)
            )
    }

    fn gen_attacks(&self, position: &StandardPosition, pce: Piece) -> Bitboard {
        match pce.kind() {
            PieceKind::Pawn => {
                let pawns = position.pieces.get_sqs(pce);
                let (left, right) = match pce.color() {
                    Color::White => (Direction::NorthWest, Direction::NorthEast),
                    Color::Black => (Direction::SouthEast, Direction::SouthWest),
                };
                (pawns >> left) | (pawns >> right)
            },
            _ => unsafe { self.gen_non_pawn_attacks(position, pce) },
        }
    }

    fn gen_danger_sqs(&self, position: &StandardPosition) -> Bitboard {
        let opponent = !position.to_move;

        self.gen_attacks(position, Piece(PieceKind::Pawn, opponent))
        | self.gen_attacks(position, Piece(PieceKind::Knight, opponent))
        | self.gen_attacks(position, Piece(PieceKind::Bishop, opponent))
        | self.gen_attacks(position, Piece(PieceKind::Rook, opponent))
        | self.gen_attacks(position, Piece(PieceKind::Queen, opponent))
        | self.gen_attacks(position, Piece(PieceKind::King, opponent))
    }

    /// # Safety
    /// `kind` can't be `Pawn`
    unsafe fn gen_non_pawn_moves(&self, kind: PieceKind, position: &StandardPosition, moves: &mut ArrayVec<[Move; 256]>) {
        let pce = Piece(kind, position.to_move);
        let pieces = position.pieces.get_sqs(pce);

        let own_occ = position.pieces.get_sqs_for(position.to_move);

        let danger_sqs;
        if pce.kind() == PieceKind::King {
            danger_sqs = self.gen_danger_sqs(position);

            Self::gen_castling_moves(position, danger_sqs, moves);
        } else {
            danger_sqs = Bitboard::new();
        }

        for from in pieces {
            let legal = self.gen_non_pawn_attacks_from_sq(pce, from) - own_occ - danger_sqs;

            for to in legal {
                moves.push(Move::Regular(from, to));
            }
        }
    }

    fn gen_castling_moves(position: &StandardPosition, danger_sqs: Bitboard, moves: &mut ArrayVec<[Move; 256]>) {
        fn gen_castling_move(position: &StandardPosition, side: Side, danger_sqs: Bitboard, moves: &mut ArrayVec<[Move; 256]>) {
            if !position.castling.get(position.to_move, side) {
                return;
            }

            use Square::*;
            let (castling_sqs, no_occ_sqs) = match (position.to_move, side) {
                (Color::White, Side::KingSide) => (bb!(E1, F1, G1), bb!(F1, G1)),
                (Color::Black, Side::KingSide) => (bb!(E8, F8, G8), bb!(F8, G8)),
                (Color::White, Side::QueenSide) => (bb!(E1, D1, C1), bb!(D1, C1, B1)),
                (Color::Black, Side::QueenSide) => (bb!(E8, D8, C8), bb!(D8, C8, B8)),
            };

            let occ = position.pieces.get_occupied();

            if ((castling_sqs & danger_sqs) | (no_occ_sqs & occ)).is_empty() {
                moves.push(Move::Castling(side));
            }
        }

        gen_castling_move(position, Side::KingSide, danger_sqs, moves);
        gen_castling_move(position, Side::QueenSide, danger_sqs, moves);
    }
}