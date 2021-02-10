use std::fmt::{Debug, Formatter};

use crate::framework::fen::FenParseError;
use crate::framework::Game;
use crate::framework::moves::{Move, MoveList};
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

    fn move_legal(&mut self, mv: Move) -> bool {
        self.move_gen.gen_all_moves(&self.position).contains(mv)
    }
}

impl Game for StandardGame {
    fn perft(&mut self, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
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
                    count += inner(game, depth - 1);
                    game.position.unmake_move();
                }
            }
            count
        }

        inner(self, depth)
    }

    fn get_moves(&mut self) -> MoveList {
        self.move_gen.gen_all_moves(&self.position)
    }

    fn make_move(&mut self, mv: Move) -> Result<(), ()> {
        if self.move_legal(mv) {
            unsafe {
                self.position.make_move(mv);
            }
            Ok(())
        } else {
            Err(())
        }
    }

    fn unmake_move(&mut self) -> Result<(), ()> {
        if self.position.last_move().is_some() {
            unsafe {
                self.position.unmake_move();
            }
            Ok(())
        } else {
            Err(())
        }
    }

    fn set_position(&mut self, fen: &str) -> Result<(), FenParseError> {
        self.position = Position::from_fen(fen)?;
        Ok(())
    }
}

impl Debug for StandardGame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.position, f)
    }
}


#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::path::PathBuf;

    use serde::Deserialize;
    use serde_json::from_reader;

    use crate::framework::Game;
    use crate::standard::game::StandardGame;

    #[derive(Deserialize)]
    struct PerftPosition {
        depth: u32,
        nodes: u64,
        fen: String,
    }

    #[test]
    fn test_perft() {
        let mut game = StandardGame::new();

        let mut test_path = PathBuf::new();
        test_path.push(env!("CARGO_MANIFEST_DIR"));
        test_path.push("resources");
        test_path.push("test");
        test_path.push("perft_positions.json");

        let test_file = File::open(test_path).unwrap();

        let tests: Vec<PerftPosition> = from_reader(test_file).unwrap();

        println!("Testing Perft..");
        for (i, test) in tests.iter().enumerate() {
            game.set_position(&test.fen).unwrap();
            println!("Running test position {}..", i + 1);
            assert_eq!(game.perft(test.depth), test.nodes);
        }
        println!("All Perft test positions passed")
    }

}