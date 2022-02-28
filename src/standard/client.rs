use std::fmt;
use std::mem::size_of;

use crate::framework::{Eval, MoveGen, Searchable, NotSupportedError};
use crate::framework::Client as ClientTrait;
use crate::framework::fen::FenParseError;
use crate::framework::moves::Move;
use crate::framework::moves::MoveList;
use crate::standard::Position;
use crate::standard::search::Search;

use super::search::transposition_table::{TranspositionTable, Entry};

const NOT_INIT: &str = "Client not initialized";

pub struct Client<MG, E> {
    position: Option<Position>,
    move_gen: Option<MG>,
    eval: Option<E>,
    trans_table: TranspositionTable,
}

impl<MG, E> Client<MG, E> where
    MG: MoveGen<Position>,
    E: Eval<Position>
{
    pub fn new() -> Self {
        let trans_table = TranspositionTable::new();
        Self {
            position: None,
            move_gen: None,
            eval: None,
            trans_table,
        }
    }

    fn move_legal(&self, mv: Move) -> bool {
        self.get_moves().contains(mv)
    }
}

impl<MG, E> crate::framework::Client for Client<MG, E> where
    MG: MoveGen<Position>,
    E: Eval<Position>
{
    fn init(&mut self) {
        self.position = Some(Position::new());
        self.move_gen = Some(MoveGen::create());
        self.eval = Some(Eval::create());
    }

    fn is_init(&self) -> bool {
        self.move_gen.is_some()
    }

    fn set_position(&mut self, fen: &str) -> Result<(), FenParseError> {
        *self.position.as_mut().expect(NOT_INIT) = Position::from_fen(fen)?;
        Ok(())
    }

    fn get_moves(&self) -> MoveList {
        self.move_gen.as_ref().expect(NOT_INIT)
            .gen_all_moves(self.position.as_ref().expect(NOT_INIT))
    }

    fn make_move(&mut self, mv: Move) -> Result<(), String> {
        if self.move_legal(mv) {
            unsafe {
                self.position.as_mut().expect(NOT_INIT)
                    .make_move(mv);
            }
            Ok(())
        } else {
            Err(format!("Illegal move: {}", mv))
        }
    }

    fn unmake_move(&mut self) -> Result<(), String> {
        let position = self.position.as_mut().expect(NOT_INIT);
        if position.last_move().is_some() {
            unsafe {
                position.unmake_move();
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

        let mut position = self.position.clone().expect(NOT_INIT);
        inner(&mut position, move_gen, depth)
    }

    /// Sets the hash size in MB
    fn set_hash_size(&mut self, hash_size: usize) -> Result<(), NotSupportedError> {
        let capacity = hash_size * (1 << 20) / size_of::<Entry>();
        self.trans_table = TranspositionTable::with_capacity(capacity);
        Ok(())
    }
}
impl<'client, MG, E> Searchable<'client> for &'client mut Client<MG, E> where
    MG: MoveGen<Position>,
    E: Eval<Position>
{
    type Search = Search<'client, MG, E>;

    fn search(&'client mut self) -> Self::Search {
        let move_gen = self.move_gen.as_ref().expect(NOT_INIT);
        let eval = self.eval.as_ref().unwrap();
        let position = self.position.clone().expect(NOT_INIT);
        self.trans_table.clear();

        Search::new(position, move_gen, eval, &mut self.trans_table)
    }
}

impl<MG, E> fmt::Debug for Client<MG, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
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
    use crate::standard::{Eval, MoveGen};

    #[derive(Deserialize)]
    struct PerftPosition {
        depth: u32,
        nodes: u64,
        fen: String,
    }

    #[test]
    fn test_perft() {
        let mut client = standard::Client::<MoveGen, Eval>::new();
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