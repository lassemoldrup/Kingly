use crate::types::Value;
use crate::Position;

mod material;
mod piece_square_tables;

pub use material::{piece_value, MaterialEval};
pub use piece_square_tables::{piece_value_early, piece_value_endgame};

/// A NegaMax (i.e. positive values represent the side to move) evaluator of
/// positions.
pub trait Eval: Clone + Send + 'static {
    fn eval(&self, position: &Position) -> Value;
}

#[derive(Clone, Copy)]
pub struct StandardEval;

impl Eval for StandardEval {
    fn eval(&self, position: &Position) -> Value {
        let game_phase = position.game_phase();
        let sign = position.to_move.sign() as i32;
        let early = sign * position.eval_early_game as i32 * game_phase;
        let endgame = sign * position.eval_endgame as i32 * (26 - game_phase);

        let mut val = ((early + endgame) / 26) as i16;
        // Tempo bonus
        val += 25;

        Value::centipawn(val)
    }
}
