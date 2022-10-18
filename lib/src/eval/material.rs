use crate::position::Position;
use crate::types::{Piece, PieceKind, Value};

use super::Eval;

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

    let piece_values = [100, 300, 300, 500, 900];
    [Pawn, Knight, Bishop, Rook, Queen]
        .into_iter()
        .zip(piece_values)
        .map(|(kind, val)| val * piece_diff(position, kind))
        .sum()
}
