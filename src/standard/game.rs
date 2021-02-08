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
    fn perft(mut self, depth: u32) -> u64 {
        if depth == 0 {
            return 0;
        }

        fn inner(game: &mut StandardGame, depth: u32) -> u64 {
            let moves = game.move_gen.gen_all_moves(&game.position);
            if depth == 1 {
                return moves.len() as u64;
            }

            let mut count = 0;
            for m in moves {
                unsafe {
                    game.position.make_move(m);
                }
                count += inner(game, depth - 1);
                game.position.unmake_move();
            }
            count
        }

        inner(&mut self, depth)
    }
}