use crate::move_gen::MoveGen;
use crate::position::Position;
use crate::tables::Tables;
use crate::types::Value;

use super::*;

#[derive(Clone, Copy)]
pub struct StandardEval {
    move_gen: MoveGen,
}

impl StandardEval {
    pub fn new(tables: &'static Tables) -> Self {
        let move_gen = MoveGen::new(tables);
        Self { move_gen }
    }
}

impl Eval for StandardEval {
    fn create() -> Self {
        Self::new(Tables::get())
    }

    fn eval(&self, position: &Position) -> Value {
        let material = get_material_score(position);

        let mobility = self.move_gen.get_mobility(position, position.to_move) as i16
            - self.move_gen.get_mobility(position, !position.to_move) as i16;

        Value::from_cp(material + 2 * mobility + 7)
    }
}
