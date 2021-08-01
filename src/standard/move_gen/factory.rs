use super::MoveGen;
use crate::framework::Position;
use crate::standard::piece_map::BitboardPieceMap;

pub struct MoveGenFactory;

impl<P: Position<PieceMap = BitboardPieceMap>> crate::framework::MoveGenFactory<MoveGen, P> for MoveGenFactory {
    fn create(&self) -> MoveGen {
        MoveGen::new()
    }
}