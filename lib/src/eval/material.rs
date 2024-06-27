use crate::position::Position;
use crate::types::{Piece, PieceKind, Value};

use super::Eval;

// See types::PieceKind for order of piece kinds
const PIECE_VALUES: [i16; 6] = [300, 300, 500, 900, 100, 0];

/// Returns the value of a given piece kind.
pub const fn piece_value(kind: PieceKind) -> i16 {
    PIECE_VALUES[kind as usize]
}

/// An evaluator that only evaluates based on material.
#[derive(Clone, Copy)]
pub struct MaterialEval;

impl Eval for MaterialEval {
    fn eval(&self, position: &Position) -> Value {
        let material = get_material_score(position);
        Value::centipawn(material)
    }
}

fn piece_diff(position: &Position, kind: PieceKind) -> i16 {
    let pieces = &position.pieces;
    let to_move = position.to_move;

    pieces.get_bb(Piece(kind, to_move)).len() as i16
        - pieces.get_bb(Piece(kind, !to_move)).len() as i16
}

/// Returns the difference in material for the given position.
pub fn get_material_score(position: &Position) -> i16 {
    use PieceKind::*;

    [Knight, Bishop, Rook, Queen, Pawn]
        .into_iter()
        .map(|kind| piece_value(kind) * piece_diff(position, kind))
        .sum()
}
