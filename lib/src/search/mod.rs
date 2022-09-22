use std::cell::RefCell;
use std::cmp::Reverse;
use std::mem;
use std::rc::Weak;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use log::error;

use crate::eval::Eval;
use crate::move_gen::MoveGen;
use crate::move_list::MoveList;
use crate::position::Position;
use crate::types::{Move, PseudoMove, Value};

use self::transposition_table::{Bound, Entry};

#[cfg(test)]
mod tests;
mod transposition_table;
pub use transposition_table::TranspositionTable;
#[cfg(feature = "trace_search")]
mod trace;
#[cfg(feature = "trace_search")]
pub use trace::{AspirationResult, Observer, ReturnKind};

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

type Callback<'f> = dyn FnMut(&SearchInfo) + 'f;
pub struct Search<'c, 'f, E> {
    limits: Limits,
    callbacks: Vec<Box<Callback<'f>>>,
    position: Position,
    move_gen: MoveGen,
    eval: E,
    trans_table: &'c mut TranspositionTable,
    #[cfg(feature = "trace_search")]
    observers: Vec<Weak<RefCell<dyn trace::Observer>>>,
}

impl<'c, 'f, E: Eval> Search<'c, 'f, E> {
    pub fn new(
        position: Position,
        move_gen: MoveGen,
        eval: E,
        trans_table: &'c mut TranspositionTable,
    ) -> Self {
        Self {
            limits: Limits::default(),
            callbacks: vec![],
            position,
            move_gen,
            eval,
            trans_table,
            observers: vec![],
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

    fn score_move(mv: &Move) -> i32 {
        todo!()
    }

    fn reorder_moves(&self, mut moves: &mut [Move], best_move: Option<Move>) {
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
            moves = &mut moves[1..];
        }

        moves.sort_unstable_by_key(Self::score_move);
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
        alpha: Value,
        beta: Value,
        depth: u8,
        params: &mut SearchParams,
        search: fn(&mut Self, Value, Value, u8, &mut SearchParams) -> Value,
    ) -> Option<Value> {
        if moves.is_empty() {
            return None;
        }

        let mut best_move = moves[0];
        let mut best_score = Value::mate_in_neg(0);
        let mut low = alpha;

        for &mv in moves {
            if self.should_stop(params) {
                return None;
            }

            #[cfg(feature = "trace_search")]
            self.notify_move_made(mv, -beta, -low);

            self.position.make_move(mv);
            let score = -search(self, -beta, -low, depth - 1, params);
            self.position.unmake_move();

            #[cfg(feature = "trace_search")]
            self.notify_move_unmade(mv);

            if score >= beta {
                let entry = Entry::new(score, mv, Bound::Lower, depth);
                self.trans_table.insert(&self.position, entry);

                // TODO: Should we notify score or -score?
                #[cfg(feature = "trace_search")]
                self.notify_score_found(score, trace::ReturnKind::Beta(mv));

                return Some(score);
            }

            if score > best_score {
                best_move = mv;
                best_score = score;
                low = low.max(score);
            }
        }

        let bound = if best_score <= alpha {
            Bound::Upper
        } else {
            Bound::Exact
        };
        let entry = Entry::new(best_score, best_move, bound, depth);
        self.trans_table.insert(&self.position, entry);

        #[cfg(feature = "trace_search")]
        self.notify_score_found(best_score, trace::ReturnKind::Best(best_move));

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

                #[cfg(feature = "trace_search")]
                self.notify_score_found(score, trace::ReturnKind::Checkmate);

                score
            // Stalemate
            } else {
                let score = Value::centi_pawn(0);

                #[cfg(feature = "trace_search")]
                self.notify_score_found(score, trace::ReturnKind::Stalemate);

                score
            };
        }

        // Draw by threefold repetition or fifty-move rule
        if self.position.is_draw() {
            let score = Value::centi_pawn(0);

            #[cfg(feature = "trace_search")]
            self.notify_score_found(score, trace::ReturnKind::RuleDraw);

            return score;
        }

        // TODO: Do this better
        let mut table_move = None;
        if let Some(entry) = self.trans_table.get(&self.position) {
            if entry.depth >= depth {
                match entry.bound {
                    Bound::Exact => {
                        #[cfg(feature = "trace_search")]
                        self.notify_score_found(entry.score, trace::ReturnKind::TTExact);

                        return entry.score;
                    }
                    Bound::Lower => alpha = alpha.max(entry.score),
                    Bound::Upper => beta = beta.min(entry.score),
                }

                if beta <= alpha {
                    #[cfg(feature = "trace_search")]
                    self.notify_score_found(entry.score, trace::ReturnKind::TTBound);

                    return entry.score;
                }
            }

            table_move = Some(entry.best_move);
        }

        if depth == 0 {
            let score = self.quiesce(alpha, beta, params.start_depth, params);
            let best_move = table_move.unwrap_or_else(|| moves[0]);
            let bound = if score <= alpha {
                Bound::Upper
            } else if score >= beta {
                Bound::Lower
            } else {
                Bound::Exact
            };

            let entry = Entry::new(score, best_move, bound, depth);
            self.trans_table.insert(&self.position, entry);

            #[cfg(feature = "trace_search")]
            self.notify_score_found(score, trace::ReturnKind::Quiesce);

            return score;
        }

        self.reorder_moves(&mut moves, table_move);

        // Safety: `moves` are generated
        unsafe {
            self.search_moves(&moves, alpha, beta, depth, params, Self::search)
                .unwrap_or(Value::mate_in_neg(0))
        }
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

            #[cfg(feature = "trace_search")]
            self.notify_aspiration_start(entry.score);

            for exp in 1.. {
                #[cfg(feature = "trace_search")]
                self.notify_aspiration_iter_start(low, high);

                let score = self.search(low, high, depth, params);
                let delta = START_DELTA * (1 << exp);

                if score >= high {
                    if score >= beta {
                        #[cfg(feature = "trace_search")]
                        self.notify_aspiration_iter_end(trace::AspirationResult::FailBeta);

                        return score;
                    }

                    #[cfg(feature = "trace_search")]
                    self.notify_aspiration_iter_end(trace::AspirationResult::FailHigh);

                    high = (score + high - entry.score).max(entry.score + delta);
                } else if score <= low {
                    if score <= alpha {
                        // This should never happen when calling from the root
                        #[cfg(feature = "trace_search")]
                        self.notify_aspiration_iter_end(trace::AspirationResult::FailAlpha);

                        return score;
                    }
                    #[cfg(feature = "trace_search")]
                    self.notify_aspiration_iter_end(trace::AspirationResult::FailLow);

                    low = (score + low - entry.score).min(entry.score - delta);
                } else {
                    #[cfg(feature = "trace_search")]
                    self.notify_aspiration_iter_end(trace::AspirationResult::InBounds);

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
            #[cfg(feature = "trace_search")]
            self.notify_new_depth(depth);

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
