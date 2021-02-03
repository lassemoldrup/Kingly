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

#[cfg(test)]
mod tests;

pub struct MoveGen {
    knight_attacks: SquareMap<Bitboard>,
}

impl MoveGen {
    pub fn new() -> Self {
        let knight_attacks = Self::init_knight_attacks();

        MoveGen {
            knight_attacks
        }
    }

    fn init_knight_attacks() -> SquareMap<Bitboard> {
        let move_vecs = [
            SquareVec(2, 1), SquareVec(1, 2), SquareVec(-1, 2), SquareVec(-2, 1),
            SquareVec(-2, -1), SquareVec(-1, -2), SquareVec(1, -2), SquareVec(2, -1),
        ];

        let mut table = SquareMap::new([Bitboard::new(); 64]);

        for (sq, ss) in table.iter_mut() {
            *ss = move_vecs.into_iter().fold(Bitboard::new(), |acc, vec| {
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
        let fwd_no_promo = fwd & !last_rank;
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
        let left_atk_no_promo = left_atk & !last_rank;
        let right_atk_no_promo = right_atk & !last_rank;
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

    fn gen_knight_moves(&self, position: &StandardPosition, moves: &mut ArrayVec<[Move; 256]>) {
        let knights = position.pieces.get_sqs(Piece(PieceKind::Knight, position.to_move));

        for from in knights {
            let own_occ = position.pieces.get_sqs_for(position.to_move);
            let legal = self.knight_attacks[from] & !own_occ;

            for to in legal {
                moves.push(Move::Regular(from, to));
            }
        }
    }
}