use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::{fmt, mem, thread};

use kingly_lib::eval::Eval;
use kingly_lib::move_gen::MoveGen;
use kingly_lib::move_list::MoveList;
use kingly_lib::position::Position;
use kingly_lib::search::{Search, SearchInfo, TranspositionTable};
use kingly_lib::tables::Tables;
use kingly_lib::types::{Move, PseudoMove};
use parking_lot::Mutex;

use crate::uci::GoOption;

#[cfg(test)]
mod tests;

const NOT_INIT: &str = "Client not initialized";

pub struct Client<E> {
    position: Option<Position>,
    move_gen: Option<MoveGen>,
    eval: Option<E>,
    trans_table: Option<Arc<Mutex<TranspositionTable>>>,
    stop_search: Arc<AtomicBool>,
    search_handle: Option<JoinHandle<()>>,
}

impl<E: Eval + Clone + Send + Sync + 'static> Client<E> {
    pub fn new() -> Self {
        let stop_search = Arc::new(AtomicBool::new(true));
        Self {
            position: None,
            move_gen: None,
            eval: None,
            trans_table: None,
            stop_search,
            search_handle: None,
        }
    }

    /// Initializes the client.
    ///
    /// We wait with initializing big tables, so the program can start quickly.
    pub fn init(&mut self) {
        self.position = Some(Position::new());
        self.move_gen = Some(MoveGen::new(Tables::get()));
        self.eval = Some(Eval::create());
        self.trans_table = Some(Arc::new(Mutex::new(TranspositionTable::new())));
    }

    pub fn is_init(&self) -> bool {
        self.move_gen.is_some()
    }

    /// Gets all legal moves in the current position
    pub fn get_moves(&self) -> MoveList {
        self.move_gen
            .as_ref()
            .expect(NOT_INIT)
            .gen_all_moves(self.position.as_ref().unwrap())
    }

    pub fn make_move(&mut self, mv: Move) -> Result<(), String> {
        if self.move_legal(mv) {
            unsafe {
                self.position.as_mut().expect(NOT_INIT).make_move(mv);
            }
            Ok(())
        } else {
            Err(format!("Illegal move '{}'", mv))
        }
    }

    pub fn unmake_move(&mut self) -> Result<(), String> {
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

    fn move_legal(&self, mv: Move) -> bool {
        self.get_moves().contains(mv)
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

        let mut position = self.position.clone().unwrap();
        inner(&mut position, move_gen, depth)
    }

    /// We are searching if there is a search thread and it is not finished
    fn is_searching(&self) -> bool {
        self.search_handle
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }

    /// UCI command: setoption name Hash value `hash_size`.
    ///
    /// `hash_size` is the number of MB.
    /// Returns `Err` if searching
    pub fn set_option_hash(&self, hash_size: usize) -> Result<(), String> {
        if self.is_searching() {
            return Err(String::from("Attempt to change hash size while searching"));
        }

        if hash_size == 0 {
            return Err(String::from("Hash size must be at least 1 MB"));
        }

        *self.trans_table.as_ref().expect(NOT_INIT).lock() =
            TranspositionTable::with_hash_size(hash_size);

        Ok(())
    }

    /// UCI command: ucinewgame
    /// Returns `Err` if searching
    pub fn uci_new_game(&mut self) -> Result<(), String> {
        if self.is_searching() {
            return Err(String::from("Attempt to set new game while searching"));
        }

        *self.position.as_mut().expect(NOT_INIT) = Position::new();
        self.trans_table.as_ref().unwrap().lock().clear();

        Ok(())
    }

    /// UCI command: position fen `fen` moves `moves`
    /// Returns `Err` if searching, `fen` is not valid, or `moves` are not legal
    pub fn position(&mut self, fen: &str, moves: &[PseudoMove]) -> Result<(), String> {
        if self.is_searching() {
            return Err(String::from("Attempt to change position while searching"));
        }

        let position = self.position.as_mut().expect(NOT_INIT);
        position.set_fen(fen).map_err(|err| err.to_string())?;

        let move_gen = self.move_gen.unwrap();
        for &mv in moves {
            let legal_moves = move_gen.gen_all_moves(position);
            let mv = mv.into_move(&legal_moves)?;
            // Safety: Move `mv` was generated, so is legal
            unsafe {
                position.make_move(mv);
            }
        }

        Ok(())
    }

    /// UCI command: go `options`.
    ///
    /// Spawns a thread and passes `on_info` into the thread.
    /// Returns `Err` if searching.
    pub fn go(
        &mut self,
        options: Vec<GoOption>,
        on_info: impl Fn(GoInfo) + Send + 'static,
    ) -> Result<(), String> {
        if self.is_searching() {
            return Err(String::from("Already searching"));
        }

        self.stop_search.store(false, Ordering::Relaxed);

        let move_gen = self.move_gen.expect(NOT_INIT);
        let eval = self.eval.clone().unwrap();
        let position = self.position.clone().unwrap();
        let trans_table = self.trans_table.as_ref().unwrap().clone();
        let stop_search = self.stop_search.clone();

        // Spawn the search thread
        self.search_handle = Some(thread::spawn(move || {
            let mut trans_table = trans_table.lock();
            let mut search = Search::new(position, move_gen, eval, &trans_table);

            for option in options {
                search = match option {
                    GoOption::SearchMoves(moves) => search.moves(&moves),
                    GoOption::Depth(depth) => search.depth(depth),
                    GoOption::Nodes(nodes) => search.nodes(nodes),
                    GoOption::MoveTime(time) => search.time(time),
                    _ => search,
                };
            }

            let mut best_move = None;
            search
                // .threads(1)
                .on_info(|res| {
                    best_move = res.pv.first().copied();
                    on_info(GoInfo::NewDepth(res));
                })
                .start(&stop_search);

            // TODO: What to do if no best move??
            if let Some(mv) = best_move {
                on_info(GoInfo::BestMove(mv));
            }

            trans_table.clear();
        }));

        Ok(())
    }

    /// UCI command: stop.
    ///
    /// Blocks until search is fully stopped and returns `Err` if not searching
    pub fn stop(&mut self) -> Result<(), String> {
        if !self.is_searching() {
            return Err(String::from("Attempt to stop while not searching"));
        }

        self.stop_search.store(true, Ordering::Relaxed);

        let mut search_handle = None;
        mem::swap(&mut search_handle, &mut self.search_handle);

        search_handle
            .unwrap()
            .join()
            .expect("Search thread panicked");

        Ok(())
    }
}

impl<E> fmt::Debug for Client<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt::Debug::fmt(&self.position, f)
    }
}

pub enum GoInfo<'a> {
    NewDepth(&'a SearchInfo),
    BestMove(Move),
}
