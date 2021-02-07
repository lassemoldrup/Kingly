use crate::framework::Game;
use crate::standard::move_gen::MoveGen;
use crate::standard::position::Position;

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