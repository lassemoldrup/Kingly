use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::eval::Eval;
use crate::move_gen::MoveGen;
use crate::move_list::MoveList;
use crate::position::Position;
use crate::types::{Move, PseudoMove, Value};

use tracing::{error, trace, trace_span};
use valuable::Valuable;

use self::transposition_table::{Bound, Entry};

#[cfg(test)]
mod tests;
mod transposition_table;
pub use transposition_table::TranspositionTable;

#[derive(Clone, Copy)]
struct SearchParams<'a> {
    nodes: u64,
    sel_depth: u8,
    start_depth: u8,
    stop_search: &'a AtomicBool,
    search_start: Instant,
}

#[derive(Default)]
struct Limits {
    moves: Option<MoveList>,
    depth: Option<u8>,
    nodes: Option<u64>,
    time: Option<Duration>,
}

pub struct Search<'client, 'f, E> {
    limits: Limits,
    callbacks: Vec<Box<dyn FnMut(&SearchInfo) + 'f>>,
    position: Position,
    move_gen: MoveGen,
    eval: E,
    trans_table: &'client mut TranspositionTable,
}

impl<'client, 'f, E: Eval> Search<'client, 'f, E> {
    pub fn new(
        position: Position,
        move_gen: MoveGen,
        eval: E,
        trans_table: &'client mut TranspositionTable,
    ) -> Self {
        Self {
            limits: Limits::default(),
            callbacks: vec![],
            position,
            move_gen,
            eval,
            trans_table,
        }
    }

    pub fn moves(mut self, moves: &[PseudoMove]) -> Self {
        let legal_moves = self.move_gen.gen_all_moves(&self.position);
        self.limits.moves = Some(
            moves
                .iter()
                .map(|&mv| mv.into_move(&legal_moves))
                .collect::<Result<_, _>>()
                .unwrap_or_else(|err| panic!("{}", err)),
        );
        self
    }

    pub fn depth(mut self, depth: u8) -> Self {
        self.limits.depth = Some(depth);
        self
    }

    pub fn time(mut self, time: Duration) -> Self {
        self.limits.time = Some(time);
        self
    }

    pub fn nodes(mut self, nodes: u64) -> Self {
        self.limits.nodes = Some(nodes);
        self
    }

    pub fn on_info(mut self, callback: impl FnMut(&SearchInfo) + 'f) -> Self {
        self.callbacks.push(Box::new(callback));
        self
    }

    fn root_moves(&mut self) -> MoveList {
        let mut search_moves = None;
        mem::swap(&mut search_moves, &mut self.limits.moves);

        search_moves.unwrap_or_else(|| self.move_gen.gen_all_moves(&self.position))
    }

    fn quiesce(
        &mut self,
        mut alpha: Value,
        beta: Value,
        sel_depth: u8,
        params: &mut SearchParams,
    ) -> Value {
        params.sel_depth = sel_depth.max(params.sel_depth);
        params.nodes += 1;

        // We assume that we can do at least as well as the static
        // eval of the current position, i.e. we don't consider zugzwang
        let static_eval = self.eval.eval(&self.position);
        if static_eval >= beta {
            return static_eval;
        } else if static_eval > alpha {
            alpha = static_eval;
        }

        let mut best_score = static_eval;

        let moves = self.move_gen.gen_captures(&self.position);
        for mv in moves {
            let score;
            // Safety: Generated moves are guaranteed to be legal
            unsafe {
                self.position.make_move(mv);
                score = -self.quiesce(-beta, -alpha, sel_depth + 1, params);
                self.position.unmake_move();
            }

            if score >= beta {
                return score;
            }

            if score > best_score {
                best_score = score;
                if score > alpha {
                    alpha = score;
                }
            }
        }

        best_score
    }

    fn reorder_moves(&self, moves: &mut [Move], best_move: Option<Move>) {
        // TODO: Do something better
        if moves.is_empty() {
            return;
        }
        if let Some(mv) = best_move {
            let mv_pos = moves.iter().position(|&m| m == mv);
            let mv_pos = match mv_pos {
                Some(pos) => pos,
                None => {
                    error!(
                        "Hash collision detected. Move: {}, Position:\n{}",
                        mv, self.position
                    );
                    return;
                }
            };

            let first = moves[0];
            moves[0] = mv;
            moves[mv_pos] = first;
        }
    }

    fn should_stop(&self, params: &SearchParams) -> bool {
        let time_searched = params.search_start.elapsed();

        params.stop_search.load(Ordering::Relaxed)
            || self.limits.time.map_or(false, |t| time_searched >= t)
            || self.limits.nodes.map_or(false, |n| params.nodes >= n)
    }

    /// Searches some `moves` and returns the best move and the value of that move
    /// # Safety
    /// `moves` must be legal
    #[inline(always)]
    unsafe fn search_moves(
        &mut self,
        moves: &[Move],
        mut alpha: Value,
        beta: Value,
        depth: u8,
        params: &mut SearchParams,
        mut search: impl FnMut(&mut Self, Value, Value, u8, &mut SearchParams) -> Value,
    ) -> Option<Value> {
        if moves.is_empty() {
            return None;
        }

        let mut best_move = moves[0];
        let mut best_score = Value::mate_in_neg(0);

        for &mv in moves {
            if self.should_stop(params) {
                return None;
            }

            #[cfg(feature = "trace")]
            let _span = trace_span!(
                "search",
                %alpha,
                %beta,
                mv = %self.position.last_move().unwrap()
            )
            .entered();

            self.position.make_move(mv);
            let score = -search(self, -beta, -alpha, depth - 1, params);
            self.position.unmake_move();

            if score >= beta {
                let entry = Entry::new(score, mv, Bound::Lower, depth);
                self.trans_table.insert(&self.position, entry);
                return Some(score);
            }

            if score > best_score {
                best_move = mv;
                best_score = score;
                alpha = alpha.max(score);
            }
        }

        let bound = if best_score < alpha {
            Bound::Upper
        } else {
            Bound::Exact
        };
        let entry = Entry::new(best_score, best_move, bound, depth);
        self.trans_table.insert(&self.position, entry);

        Some(best_score)
    }

    fn search(
        &mut self,
        mut alpha: Value,
        mut beta: Value,
        depth: u8,
        params: &mut SearchParams,
    ) -> Value {
        let (mut moves, check) = self.move_gen.gen_all_moves_and_check(&self.position);

        if moves.is_empty() {
            // Checkmate
            return if check {
                let score = Value::mate_in_neg(((params.start_depth - depth + 1) / 2) as u16);
                #[cfg(feature = "trace")]
                trace!(%score, "cm");
                score
            // Stalemate
            } else {
                let score = Value::centi_pawn(0);
                #[cfg(feature = "trace")]
                trace!(%score, "sm");
                score
            };
        }

        // Draw by threefold repetition or fifty-move rule
        if self.position.is_draw() {
            return Value::centi_pawn(0);
        }

        // TODO: Do this better
        let mut table_move = None;
        if let Some(entry) = self.trans_table.get(&self.position) {
            if entry.depth >= depth {
                match entry.bound {
                    Bound::Exact => {
                        #[cfg(feature = "trace")]
                        trace!(score = %entry.score, "tte");
                        return entry.score;
                    }
                    Bound::Lower => alpha = alpha.max(entry.score),
                    Bound::Upper => beta = beta.min(entry.score),
                }

                if beta <= alpha {
                    #[cfg(feature = "trace")]
                    trace!(score = %entry.score, "ttb");
                    return entry.score;
                }
            }

            table_move = Some(entry.best_move);
        }

        if depth == 0 {
            let score = self.quiesce(alpha, beta, params.start_depth, params);

            let best_move = table_move.unwrap_or_else(|| moves[0]);
            let entry = Entry::new(score, best_move, Bound::Exact, depth);
            self.trans_table.insert(&self.position, entry);

            #[cfg(feature = "trace")]
            trace!(%score, "qui");
            return score;
        }

        self.reorder_moves(&mut moves, table_move);
        // Safety: `moves` are generated
        let score = unsafe {
            self.search_moves(&moves, alpha, beta, depth, params, Self::search)
                .unwrap_or(Value::mate_in_neg(0))
        };

        #[cfg(feature = "trace")]
        trace!("bst");
        score
    }

    fn aspiration_window_search(
        &mut self,
        alpha: Value,
        beta: Value,
        depth: u8,
        params: &mut SearchParams,
    ) -> Value {
        if let Some(&entry) = self.trans_table.get(&self.position) {
            const START_DELTA: Value = Value::centi_pawn(12);
            let mut low = alpha.max(entry.score - START_DELTA);
            let mut high = beta.min(entry.score + START_DELTA);

            while high <= low {
                low = alpha.max(low - START_DELTA);
                high = beta.min(high + START_DELTA);
            }

            #[cfg(feature = "trace")]
            trace!(
                "Doing aspiration window search. Prev score: {}",
                entry.score
            );

            for exp in 1.. {
                let score = self.search(low, high, depth, params);
                let delta = START_DELTA * (1 << exp);

                if score >= high {
                    if score >= beta {
                        #[cfg(feature = "trace")]
                        trace!("Failed higher than β. score: {}, β: {}", score, beta);
                        return score;
                    }
                    #[cfg(feature = "trace")]
                    trace!("Fail high. score: {}, low: {}, high: {}", score, low, high);
                    high = (score + high - entry.score).max(entry.score + delta);
                } else if score < low {
                    if score < alpha {
                        // This should never happen when calling from the root
                        #[cfg(feature = "trace")]
                        trace!("Failed lower than α. score: {}, α: {}", score, alpha);
                        return score;
                    }
                    #[cfg(feature = "trace")]
                    trace!("Fail low. score: {}, low: {}, high: {}", score, low, high);
                    low = (score + low - entry.score).min(entry.score - delta);
                } else {
                    #[cfg(feature = "trace")]
                    trace!("In bounds. score: {}, low: {}, high: {}", score, low, high);
                    return score;
                }
            }
        }

        self.search(alpha, beta, depth, params)
    }

    fn primary_variation(&mut self) -> Vec<Move> {
        let mut primary_variation = vec![];

        while let Some(entry) = self.trans_table.get(&self.position) {
            // Sanity checking in case of hash collision
            let moves = self.move_gen.gen_all_moves(&self.position);
            if !moves.contains(entry.best_move) {
                error!(
                    "Hash collision detected. Move: {}, Position:\n{}",
                    entry.best_move, self.position
                );
                break;
            }

            primary_variation.push(entry.best_move);
            // Safety: Move was checked to be legal
            unsafe {
                self.position.make_move(entry.best_move);
            }
        }

        for _ in &primary_variation {
            // Safety: A move was made for each move in `primary_variation`
            unsafe {
                self.position.unmake_move();
            }
        }

        primary_variation
    }

    fn notify_info(
        &mut self,
        search_start: Instant,
        iteration_start: Instant,
        depth: u8,
        best_score: Value,
        params: SearchParams,
    ) {
        let pv = self.primary_variation();
        let hash_full = ((self.trans_table.len() * 1000) / self.trans_table.capacity()) as u32;
        let elapsed_nanos = iteration_start.elapsed().as_nanos();
        let nps = (params.nodes as u128 * 1_000_000_000 / elapsed_nanos) as u64;

        let info = SearchInfo {
            score: best_score,
            pv,
            depth,
            sel_depth: params.sel_depth,
            nodes_searched: params.nodes,
            nps,
            total_duration: search_start.elapsed(),
            hash_full,
        };

        for callback in &mut self.callbacks {
            callback(&info);
        }
    }

    /// Searches the current position with iterative deepening and notifies
    /// all callbacks with some search info at the end of each iteration.
    /// If `stop_search` is set to `true`, the search will stop.
    pub fn start(mut self, stop_search: &AtomicBool) {
        let search_start = Instant::now();

        let mut root_moves = self.root_moves();
        let max_depth = self.limits.depth.unwrap_or(u8::MAX);

        // Iterative deepening
        for depth in 1..=max_depth {
            let iteration_start = Instant::now();

            let mut params = SearchParams {
                nodes: 0,
                sel_depth: 0,
                start_depth: depth,
                stop_search,
                search_start,
            };

            let best_move = self.trans_table.get(&self.position).map(|e| e.best_move);
            self.reorder_moves(&mut root_moves, best_move);
            // Safety: `root_moves` is either generated or checked in `self.root_moves()`
            let best = unsafe {
                self.search_moves(
                    &root_moves,
                    Value::mate_in_neg(0),
                    Value::mate_in(0),
                    depth,
                    &mut params,
                    Self::aspiration_window_search,
                )
            };
            let best_score = match best {
                Some(b) => b,
                None => return,
            };

            if self.should_stop(&params) {
                return;
            }

            self.notify_info(search_start, iteration_start, depth, best_score, params);
        }
    }
}

#[derive(Clone)]
pub struct SearchInfo {
    pub score: Value,
    pub pv: Vec<Move>,
    pub depth: u8,
    pub sel_depth: u8,
    pub nodes_searched: u64,
    pub nps: u64,
    pub total_duration: Duration,
    pub hash_full: u32,
}
