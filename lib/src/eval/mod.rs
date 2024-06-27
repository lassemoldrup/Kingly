use crate::types::Value;
use crate::Position;

mod material;

pub use material::{piece_value, MaterialEval};

/// A NegaMax (i.e. positive values represent the side to move) evaluator of
/// positions.
pub trait Eval {
    fn eval(&self, position: &Position) -> Value;
}
