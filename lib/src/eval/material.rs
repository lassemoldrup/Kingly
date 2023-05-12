use crate::position::Position;
use crate::types::{Piece, PieceKind, Value};

use super::Eval;

// See types::PieceKind for order of piece kinds
pub const PIECE_VALUES: [i16; 6] = [300, 300, 500, 900, 100, 0];

/// Only evaluates based on material.
pub struct MaterialEval;

impl Eval for MaterialEval {
    fn create() -> Self {
        Self
    }

    fn eval(&self, position: &Position) -> Value {
        let material = get_material_score(position);

        Value::centi_pawn(material)
    }
}

fn piece_diff(position: &Position, kind: PieceKind) -> i16 {
    let pieces = position.pieces;
    let to_move = position.to_move;

    pieces.get_bb(Piece(kind, to_move)).len() as i16
        - pieces.get_bb(Piece(kind, !to_move)).len() as i16
}

pub fn get_material_score(position: &Position) -> i16 {
    use PieceKind::*;

    [Knight, Bishop, Rook, Queen, Pawn]
        .into_iter()
        .zip(PIECE_VALUES)
        .map(|(kind, val)| val * piece_diff(position, kind))
        .sum()
}
