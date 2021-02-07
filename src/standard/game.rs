use crate::standard::position::Position;
use crate::standard::move_gen::MoveGen;
use crate::framework::Game;

pub struct StandardGame {
    position: Position,
    move_gen: MoveGen,
}

impl StandardGame {
    pub fn new() -> Self {
        let position = Position::new();
        let move_gen = MoveGen::new();

        Self {
            position,
            move_gen,
        }
    }
}

impl Game for StandardGame {
    fn perft(depth: u32) -> u64 {
        unimplemented!()
    }
}