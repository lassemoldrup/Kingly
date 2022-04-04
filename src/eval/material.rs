use crate::position::Position;
use crate::types::Value;

use super::{get_material_score, Eval};

/// Only evaluates based on material
pub struct MaterialEval;

impl Eval for MaterialEval {
    fn create() -> Self {
        Self
    }

    fn eval(&self, position: &Position) -> Value {
        let material = get_material_score(position);

        Value::from_cp(material)
    }
}
