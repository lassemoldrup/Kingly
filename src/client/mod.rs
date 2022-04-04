use std::fmt;

use crate::eval::Eval;
use crate::fen::FenParseError;
use crate::move_gen::MoveGen;
use crate::move_list::MoveList;
use crate::position::Position;
use crate::search::{Search, TranspositionTable};
use crate::tables::Tables;
use crate::types::Move;

#[cfg(test)]
mod tests;

const NOT_INIT: &str = "Client not initialized";

pub struct Client<E> {
    position: Option<Position>,
    move_gen: Option<MoveGen>,
    eval: Option<E>,
    trans_table: TranspositionTable,
}

impl<E: Eval> Client<E> {
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

    pub fn init(&mut self) {
        self.position = Some(Position::new());
        self.move_gen = Some(MoveGen::new(Tables::get()));
        self.eval = Some(Eval::create());
    }

    pub fn is_init(&self) -> bool {
        self.move_gen.is_some()
    }

    fn new_game(&mut self) {
        let position = self.position.as_mut().expect(NOT_INIT);

        *position = Position::new();

        self.clear_trans_table();
    }

    fn set_position(&mut self, fen: &str) -> Result<(), FenParseError> {
        self.position.as_mut().expect(NOT_INIT).set_fen(fen)
    }

    fn get_moves(&self) -> MoveList {
        self.move_gen
            .as_ref()
            .expect(NOT_INIT)
            .gen_all_moves(self.position.as_ref().expect(NOT_INIT))
    }

    fn make_move(&mut self, mv: Move) -> Result<(), String> {
        if self.move_legal(mv) {
            unsafe {
                self.position.as_mut().expect(NOT_INIT).make_move(mv);
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

    pub fn perft(&self, depth: u32) -> u64 {
        let move_gen = self.move_gen.as_ref().expect(NOT_INIT);

        if depth == 0 {
            return 1;
        }

        fn inner(position: &mut Position, move_gen: &MoveGen, depth: u32) -> u64 {
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

    fn search<'client, 'f>(&'client mut self) -> Search<'client, 'f, E> {
        let move_gen = self.move_gen.as_ref().expect(NOT_INIT);
        let eval = self.eval.as_ref().unwrap();
        let position = self.position.clone().unwrap();

        Search::new(position, move_gen, eval, &mut self.trans_table)
    }

    fn clear_trans_table(&mut self) {
        self.trans_table.clear();
    }

    /// Sets the hash size in MB
    fn set_hash_size(&mut self, hash_size: usize) {
        self.trans_table = TranspositionTable::with_hash_size(hash_size);
    }
}

impl<E> fmt::Debug for Client<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt::Debug::fmt(&self.position, f)
    }
}
