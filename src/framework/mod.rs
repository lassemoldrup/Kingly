use self::square::Square;
use self::piece::Piece;
use self::moves::Move;
use crate::framework::fen::FenParseError;
use crate::framework::color::Color;
use crate::framework::moves::MoveList;

pub mod square;
pub mod color;
pub mod piece;
pub mod moves;
pub mod fen;
pub mod direction;
pub mod square_map;
pub mod square_vec;

pub trait Position {
    /// Creates default chess starting `Position`
    fn new() -> Self;
    /// Creates `Position` from `fen`
    fn from_fen(fen: &str) -> Result<Self, FenParseError> where Self: Sized;
    /// Generates all legal moves in the `Position`
    fn gen_moves(&self) -> MoveList;
    /// Makes move `m`
    fn make_move(&mut self, m: Move);
    /// Unmakes last move
    fn unmake_move(&mut self);
    /// Returns Crusty's current evaluation of the position
    fn evaluate(&self) -> i32;
}

/*pub trait SquareSet : Sized + IntoIterator<Item = Square> + BitOr<Output = Self> + BitAnd<Output = Self> + Not<Output = Self> + Shr<Direction, Output = Self> {
    /// Ranks from 1 to 8
    const RANKS: [Self; 8];
    /// Files from a to h
    const FILES: [Self; 8];
    /// Creates an empty `SquareSet`
    fn new() -> Self;
    /// Creates a `SquareSet` with `Square` `sq`
    fn from_sq(sq: Square) -> Self;
    /// Adds `Square` `sq` to the `SquareSet`
    fn add(&mut self, sq: Square);
    /// Returns whether the `SquareSet` is empty or not
    fn is_empty(&self) -> bool;
}*/

pub trait PieceMap {
    /// Creates an empty `PieceMap`
    fn new() -> Self;
    /// Sets the given `Square` `sq` to contain the `Piece` `pce`
    fn set_sq(&mut self, sq: Square, pce: Piece);
    /// Gets the `Piece` at square `sq`
    fn get(&self, sq: Square) -> Option<Piece>;
}

pub trait CastlingRights {
    /// Creates `CastlingRights` with the following castling rights: white king side `w_king`,
    /// white queen side `w_queen`, black king side `b_king` and black queen side `b_queen`
    fn new(w_king: bool, w_queen: bool, b_king: bool, b_queen: bool) -> Self;
    /// Gets castling right for `Color` `col` and `Side` `side`
    fn get(&self, col: Color, side: Side) -> bool;
    /// Sets castling right for `Color` `col` and `Side` `side` based on `value`
    fn set(&mut self, col: Color, side: Side, value: bool);
}

pub enum Side {
    KingSide, QueenSide
}