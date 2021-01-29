use crate::standard::position::StandardPosition;
use crate::framework::SquareSet;
use arrayvec::ArrayVec;
use crate::framework::moves::Move;
use crate::framework::color::Color;
use crate::framework::direction::Direction;
use crate::framework::piece::{PieceKind, Piece};

#[cfg(test)]
mod tests;

impl<S: SquareSet + Copy> StandardPosition<S> {
    fn gen_pawn_moves(&self, moves: &mut ArrayVec<[Move; 256]>) {
        let pawns = self.pieces.get_sqs(Piece(PieceKind::Pawn, self.to_move));
        let (up, up_left, up_right, fourth_rank, last_rank) = match self.to_move {
            Color::White => (Direction::North, Direction::NorthWest, Direction::NorthEast, S::RANKS[3], S::RANKS[7]),
            Color::Black => (Direction::South, Direction::SouthEast, Direction::SouthWest, S::RANKS[4], S::RANKS[0]),
        };

        fn add_regulars<S: SquareSet + Copy>(sq_set: S, dir: Direction, moves: &mut ArrayVec<[Move; 256]>) {
            sq_set.into_iter()
                .for_each(|sq| moves.push(Move::Regular(sq << dir, sq)));
        }

        fn add_promos<S: SquareSet + Copy>(sq_set: S, dir: Direction, moves: &mut ArrayVec<[Move; 256]>) {
            sq_set.into_iter()
                .for_each(|sq| {
                    let orig = sq << dir;
                    moves.push(Move::Promotion(orig, sq, PieceKind::Queen));
                    moves.push(Move::Promotion(orig, sq, PieceKind::Rook));
                    moves.push(Move::Promotion(orig, sq, PieceKind::Bishop));
                    moves.push(Move::Promotion(orig, sq, PieceKind::Knight));
                });
        }

        // Forward
        let not_occ = !self.pieces.get_occupied();
        let fwd = (pawns >> up) & not_occ;
        let fwd_no_promo = fwd & !last_rank;
        let fwd_promo = fwd & last_rank;
        let fwd2 = (fwd >> up) & fourth_rank & not_occ;

        add_regulars(fwd_no_promo, up, moves);
        add_promos(fwd_promo, up, moves);
        fwd2.into_iter()
            .for_each(|sq| moves.push(Move::Regular(sq << up << up, sq)));

        // Attacks
        let opponent_occ = self.pieces.get_sqs_for(!self.to_move);
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
        if let Some(sq) = self.en_passant_sq {
            let ep_square_set = S::from_sq(sq);

            if !(left & ep_square_set).is_empty() {
                moves.push(Move::EnPassant(sq << up_left, sq));
            }
            if !(right & ep_square_set).is_empty() {
                moves.push(Move::EnPassant(sq << up_right, sq));
            }
        }
    }
}