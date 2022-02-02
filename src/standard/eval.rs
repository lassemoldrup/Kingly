use crate::framework::piece::{Piece, PieceKind};
use crate::framework::Position;
use crate::framework::value::Value;
use crate::standard::piece_map::BitboardPieceMap;
use crate::standard::tables::Tables;
use crate::standard::MoveGen;

#[cfg(test)]
mod tests;

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

impl<P: Position<PieceMap = BitboardPieceMap>> crate::framework::Eval<P> for Eval {
    fn create() -> Self {
        Self::new(Tables::get())
    }

    fn eval(&self, position: &P) -> Value {
        let material = get_material_score(position);

        let mobility = self.move_gen.get_mobility(position, position.to_move()) as i32
            - self.move_gen.get_mobility(position, !position.to_move()) as i32;

        Value::CentiPawn(material)// + 2 * mobility)
    }
}

fn get_material_score<P>(position: &P) -> i32 where
    P: Position<PieceMap = BitboardPieceMap>
{
    use PieceKind::*;

    let piece_values = [100, 300, 300, 500, 900];
    [Pawn, Knight, Bishop, Rook, Queen].into_iter()
        .zip(piece_values)
        .map(|(kind, val)| val * piece_diff(position, kind))
        .sum()
}

fn piece_diff<P>(position: &P, kind: PieceKind) -> i32 where
    P: Position<PieceMap = BitboardPieceMap>
{
    let pieces = position.pieces();
    let to_move = position.to_move();

    pieces.get_bb(Piece(kind, to_move)).len() as i32
        - pieces.get_bb(Piece(kind, !to_move)).len() as i32
}
