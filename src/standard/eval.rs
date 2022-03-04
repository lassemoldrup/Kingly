use crate::framework::Position as _;
use crate::framework::piece::{Piece, PieceKind};
use crate::framework::value::Value;
use crate::standard::tables::Tables;
use crate::standard::MoveGen;

use super::Position;

#[cfg(test)]
mod tests;

/// NegaMax evaluation of the position
pub struct Eval {
    move_gen: MoveGen,
}

impl Eval {
    pub fn new(tables: &'static Tables) -> Self {
        let move_gen = MoveGen::new(tables);
        Self {
            move_gen
        }
    }
}

impl crate::framework::Eval<Position> for Eval {
    fn create() -> Self {
        Self::new(Tables::get())
    }

    fn eval(&self, position: &Position) -> Value {
        let material = get_material_score(position);

        let mobility = self.move_gen.get_mobility(position, position.to_move()) as i16
            - self.move_gen.get_mobility(position, !position.to_move()) as i16;

        Value::from_cp(material + 2 * mobility + 7)
    }
}

/// Only evaluates based on material
pub struct MaterialEval;

impl crate::framework::Eval<Position> for MaterialEval {
    fn create() -> Self {
        Self
    }

    fn eval(&self, position: &Position) -> Value {
        let material = get_material_score(position);

        Value::from_cp(material)
    }
}

fn get_material_score(position: &Position) -> i16 {
    use PieceKind::*;

    let piece_values = [100, 300, 300, 500, 900];
    [Pawn, Knight, Bishop, Rook, Queen].into_iter()
        .zip(piece_values)
        .map(|(kind, val)| val * piece_diff(position, kind))
        .sum()
}

fn piece_diff(position: &Position, kind: PieceKind) -> i16 {
    let pieces = position.pieces();
    let to_move = position.to_move();

    pieces.get_bb(Piece(kind, to_move)).len() as i16
        - pieces.get_bb(Piece(kind, !to_move)).len() as i16
}
