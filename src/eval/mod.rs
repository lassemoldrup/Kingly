use crate::position::Position;
use crate::types::{Piece, PieceKind, Value};

mod material;
#[cfg(test)]
pub use material::MaterialEval;
mod standard;
pub use standard::StandardEval;
#[cfg(test)]
mod tests;

/// NegaMax evaluation of the position
pub trait Eval {
    fn create() -> Self;
    fn eval(&self, position: &Position) -> Value;
}

fn piece_diff(position: &Position, kind: PieceKind) -> i16 {
    let pieces = position.pieces;
    let to_move = position.to_move;

    pieces.get_bb(Piece(kind, to_move)).len() as i16
        - pieces.get_bb(Piece(kind, !to_move)).len() as i16
}

fn get_material_score(position: &Position) -> i16 {
    use PieceKind::*;

    let piece_values = [100, 300, 300, 500, 900];
    [Pawn, Knight, Bishop, Rook, Queen]
        .into_iter()
        .zip(piece_values)
        .map(|(kind, val)| val * piece_diff(position, kind))
        .sum()
}
