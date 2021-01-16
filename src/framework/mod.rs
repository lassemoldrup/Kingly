use self::square::Square;
use self::piece::Piece;
use self::moves::Move;
use crate::framework::fen::FenParseError;

pub mod square;
pub mod color;
pub mod piece;
pub mod moves;
pub mod fen;

pub trait Position {
    /// Creates default chess starting `Position`
    fn new() -> Self;
    /// Creates `Position` from `fen`
    fn from_fen(fen: &str) -> Result<Self, FenParseError> where Self: Sized;
    /// Generates all legal moves in the `Position`
    fn gen_moves(&self) -> Vec<Move>;
    /// Makes move `m`
    fn make_move(&mut self, m: Move);
    /// Unmakes last move
    fn unmake_move(&mut self);
    /// Returns Crusty's current evaluation of the position
    fn evaluate(&self) -> i32;
}

pub trait SquareSet : IntoIterator<Item = Square> {
    /// Creates an empty `SquareSet`
    fn new() -> Self;
    /// Adds `Square` `sq` to the `SquareSet`
    fn add(&mut self, sq: Square);
}

pub trait PieceMap {
    /// Creates an empty `PieceMap`
    fn new() -> Self;
    /// Sets the given `Square` `sq` to contain the `Piece` `pce`
    fn set_sq(&mut self, sq: Square, pce: Piece);
    /// Gets the `Piece` at square `sq`
    fn get(&self, sq: Square) -> Option<Piece>;
}

