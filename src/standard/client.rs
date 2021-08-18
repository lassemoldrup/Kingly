use std::fmt;

use crate::framework::{Eval, MoveGen, MoveGenFactory, Searchable};
use crate::framework::Client as ClientTrait;
use crate::framework::fen::FenParseError;
use crate::framework::moves::Move;
use crate::framework::moves::MoveList;
use crate::standard::Position;
use crate::standard::search::Search;

const NOT_INIT: &str = "Client not initialized";

pub struct Client<MG, MGF, E>
{
    position: Position,
    move_gen: Option<MG>,
    move_gen_factory: MGF,
    eval: E,
}

impl<MG, MGF, E> Client<MG, MGF, E> where
    MG: MoveGen<Position>,
    MGF: MoveGenFactory<MG, Position>,
    E: Eval<Position>
{
    pub fn new(move_gen_factory: MGF, eval: E) -> Self {
        Self {
            position: Position::new(),
            move_gen: None,
            move_gen_factory,
            eval,
        }
    }

    fn move_legal(&self, mv: Move) -> bool {
        self.get_moves().contains(mv)
    }
}

impl<MG, MGF, E> crate::framework::Client for Client<MG, MGF, E> where
    MG: MoveGen<Position>,
    MGF: MoveGenFactory<MG, Position>,
    E: Eval<Position>
{
    fn init(&mut self) {
        self.move_gen = Some(self.move_gen_factory.create());
    }

    fn is_init(&self) -> bool {
        self.move_gen.is_some()
    }

    fn set_position(&mut self, fen: &str) -> Result<(), FenParseError> {
        self.position = Position::from_fen(fen)?;
        Ok(())
    }

    fn get_moves(&self) -> MoveList {
        self.move_gen.as_ref().expect(NOT_INIT)
            .gen_all_moves(&self.position)
    }

    fn make_move(&mut self, mv: Move) -> Result<(), String> {
        if self.move_legal(mv) {
            unsafe {
                self.position.make_move(mv);
            }
            Ok(())
        } else {
            Err(format!("Illegal move: {}", mv))
        }
    }

    fn unmake_move(&mut self) -> Result<(), String> {
        if self.position.last_move().is_some() {
            unsafe {
                self.position.unmake_move();
            }
            Ok(())
        } else {
            Err("No move to unmake".to_string())
        }
    }

    fn perft(&self, depth: u32) -> u64 {
        let move_gen = self.move_gen.as_ref().expect(NOT_INIT);

        if depth == 0 {
            return 1;
        }

        fn inner<MG: MoveGen<Position>>(position: &mut Position, move_gen: &MG, depth: u32) -> u64 {
            let moves = move_gen.gen_all_moves(position);
            if depth == 1 {
                return moves.len() as u64;
            }

            let mut count = 0;
            for m in moves {
                unsafe {
                    position.make_move(m);
                    count += inner(position, move_gen, depth - 1);
                    position.unmake_move();
                }
            }

            count
        }

        inner(&mut self.position.clone(), move_gen, depth)
    }
}

impl<'client, MG, MGF, E> Searchable<'client> for &'client Client<MG, MGF, E> where
    MG: MoveGen<Position>,
    MGF: MoveGenFactory<MG, Position>,
    E: Eval<Position>
{
    type InfSearch = Search<'client, MG, E>;
    type DepthSearch = Search<'client, MG, E>;

    fn search_depth(&self, depth: u32) -> Self::DepthSearch {
        todo!()
    }

    fn search(&self) -> Self::InfSearch {
        let move_gen = self.move_gen.as_ref().expect(NOT_INIT);
        let position = self.position.clone();

        Search::new(position, move_gen, &self.eval)
    }
}

impl<MG, MGF, E> fmt::Debug for Client<MG, MGF, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        fmt::Debug::fmt(&self.position, f)
    }
}


#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::path::PathBuf;

    use serde::Deserialize;
    use serde_json::from_reader;

    use crate::framework::Client;
    use crate::standard;
    use crate::standard::Eval;
    use crate::standard::move_gen::MoveGenFactory;

    #[derive(Deserialize)]
    struct PerftPosition {
        depth: u32,
        nodes: u64,
        fen: String,
    }

    #[test]
    fn test_perft() {
        let mut client = standard::Client::new(MoveGenFactory, Eval);
        client.init();

        let mut test_path = PathBuf::new();
        test_path.push(env!("CARGO_MANIFEST_DIR"));
        test_path.push("resources");
        test_path.push("test");
        test_path.push("perft_positions.json");

        let test_file = File::open(test_path).unwrap();

        let tests: Vec<PerftPosition> = from_reader(test_file).unwrap();

        println!("Testing Perft..");
        for (i, test) in tests.iter().enumerate() {
            client.set_position(&test.fen).unwrap();
            println!("Running test position {}..", i + 1);
            assert_eq!(client.perft(test.depth), test.nodes);
        }
        println!("All Perft test positions passed")
    }

}