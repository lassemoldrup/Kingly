use crate::position::Position;
use crate::types::Value;

mod material;
#[cfg(test)]
pub use material::MaterialEval;
mod standard;
pub use standard::StandardEval;
#[cfg(test)]
mod tests;

/// NegaMax evaluation of the position
pub trait Eval {
    fn create() -> Self;
    fn eval(&self, position: &Position) -> Value;
}
