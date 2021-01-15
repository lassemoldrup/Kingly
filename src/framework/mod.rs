use self::square::Square;
use self::piece::{Piece, PieceKind};
use self::fen::Fen;
use self::moves::Move;

pub mod square;
pub mod color;
pub mod piece;
pub mod fen;
pub mod moves;

pub trait Position {
    /// Creates default chess starting `Position`
    fn new() -> Self;
    /// Creates `Position` from `fen`
    fn from_fen(fen: &Fen) -> Self;
    /// Generates all legal moves in the `Position`
    fn gen_moves(&self) -> Vec<Move>;
    /// Makes move `m`
    fn make_move(&mut self, m: Move);
    /// Unmakes last move
    fn unmake_move(&mut self);
    /// Returns Crusty's current evaluation of the position
    fn evaluate(&self) -> i32;
}

pub trait SquareSet : IntoIterator<Item = Square> { }

pub trait PieceMap {
    /// Sets the given `Square` `sq` to contain the `Piece` `pce`
    fn set_square(&mut self, sq: Square, pce: Piece);
    /// Gets the `Piece` at square `sq`
    fn get(&self, sq: Square) -> Piece;
}