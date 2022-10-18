use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use log::error;

use crate::eval::Eval;
use crate::move_gen::MoveGen;
use crate::move_list::MoveList;
use crate::mv;
use crate::position::Position;
use crate::types::{Move, PseudoMove, Value};

use self::transposition_table::{Bound, Entry};

#[cfg(test)]
mod tests;
mod transposition_table;
pub use transposition_table::TranspositionTable;
mod trace;
pub use trace::{AspirationResult, Observer, ReturnKind};

#[derive(Clone, Copy, Debug)]
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

/// Represents a handle to a search of some `Position`.
///
/// A `Search` can be used to build up a search with specific limits such as a
/// max search depth. The actual search is executed upon calling the `start`
/// method, once the `Search` has been initialized with the desired limits.
pub struct Search<'c, 'f, E, O = ()> {
    limits: Limits,
    callbacks: Vec<Box<Callback<'f>>>,
    position: Position,
    move_gen: MoveGen,
    eval: E,
    trans_table: &'c TranspositionTable,
    observer: O,
}

impl<'c, 'f, E: Eval> Search<'c, 'f, E> {
    pub fn new(
        position: Position,
        move_gen: MoveGen,
        eval: E,
        trans_table: &'c TranspositionTable,
    ) -> Self {
        Self {
            limits: Limits::default(),
            callbacks: vec![],
            position,
            move_gen,
            eval,
            trans_table,
            observer: (),
        }
    }
}

impl<'c, 'f, E: Eval, O: Observer> Search<'c, 'f, E, O> {
    pub fn moves(mut self, moves: &[PseudoMove]) -> Self {
        let legal_moves = self.move_gen.gen_all_moves(&self.position);
        let moves = moves
            .iter()
            .map(|&mv| mv.into_move(&legal_moves).unwrap())
            .collect();
        self.limits.moves = Some(moves);
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
        let search_moves = self.limits.moves.take();
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
                params.nodes += 1;
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

    fn score_move(mv: &Move) -> impl Ord {
        !mv.capture()
    }

    fn reorder_moves(&self, mut moves: &mut [Move], best_move: Option<Move>) {
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

    /// Searches some `moves` and returns the value of the best move.
    /// # Safety
    /// `moves` must be legal.
    #[inline(always)]
    unsafe fn search_moves(
        &mut self,
        moves: &[Move],
        alpha: Value,
        beta: Value,
        depth: i8,
        params: &mut SearchParams,
        search: fn(&mut Self, Value, Value, i8, &mut SearchParams) -> Value,
    ) -> Option<Value> {
        if moves.is_empty() {
            return None;
        }

        let mut best_move = moves[0];
        let mut best_score = Value::mate_in_ply_neg(0);
        let mut low = alpha;

        for &mv in moves {
            if self.should_stop(params) {
                return None;
            }

            // Calling dec_mate and inc_mate, since mate in 5 ply
            // at this depth will be mate in 4 ply at child depth
            self.notify_move_made(mv, -beta.dec_mate(), -low.dec_mate());

            self.position.make_move(mv);
            params.nodes += 1;
            let score =
                -search(self, -beta.dec_mate(), -low.dec_mate(), depth - 1, params).inc_mate();
            self.position.unmake_move();

            self.notify_move_unmade(mv);

            if score >= beta {
                let entry = Entry::new(score, mv, Bound::Lower, depth);
                self.trans_table.insert(&self.position, entry);

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

        self.notify_score_found(best_score, trace::ReturnKind::Best(best_move));

        Some(best_score)
    }

    /// Returns `Some(score)` if the current position can be pruned at `depth`,
    /// where `score` is the estimated value of the position.
    /// This includes doing quiescence search as well. Otherwise, returns `None`.
    fn prune(
        &mut self,
        alpha: Value,
        beta: Value,
        depth: i8,
        best_move: Move,
        params: &mut SearchParams,
    ) -> Option<Value> {
        // Do quiescence search
        if depth <= 0 {
            let score = self.quiesce(alpha, beta, params.start_depth + (-depth) as u8, params);
            let bound = if score <= alpha {
                Bound::Upper
            } else if score >= beta {
                Bound::Lower
            } else {
                Bound::Exact
            };

            let entry = Entry::new(score, best_move, bound, depth);
            self.trans_table.insert(&self.position, entry);

            self.notify_score_found(score, trace::ReturnKind::Quiesce);

            return Some(score);
        }

        // Null move pruning with R=2
        if depth > 2 && self.position.null_move_heuristic() {
            self.notify_move_made(mv!(), -beta.dec_mate(), -alpha.dec_mate());

            let score;
            unsafe {
                self.position.make_move(mv!());
                params.nodes += 1;
                score = -self
                    .search(-beta.dec_mate(), -alpha.dec_mate(), depth - 2, params)
                    .inc_mate();
                self.position.unmake_move();
            }

            self.notify_move_unmade(mv!());

            if score >= beta {
                let entry = Entry::new(score, best_move, Bound::Lower, depth);
                self.trans_table.insert(&self.position, entry);

                self.notify_score_found(score, trace::ReturnKind::NullMove);

                return Some(score);
            }
        }

        None
    }

    /// Searches the current position to `depth` with negamax alpha-beta search
    /// (https://www.chessprogramming.org/Alpha-Beta). Currently, the engine uses
    /// fail-soft, which means it may return values outside of the interval `(alpha; beta)`.
    fn search(
        &mut self,
        mut alpha: Value,
        mut beta: Value,
        depth: i8,
        params: &mut SearchParams,
    ) -> Value {
        let (mut moves, check) = self.move_gen.gen_all_moves_and_check(&self.position);

        if moves.is_empty() {
            // Checkmate
            return if check {
                let score = Value::mate_in_ply_neg(0);

                self.notify_score_found(score, trace::ReturnKind::Checkmate);

                score
            // Stalemate
            } else {
                let score = Value::centi_pawn(0);

                self.notify_score_found(score, trace::ReturnKind::Stalemate);

                score
            };
        }

        // Draw by threefold repetition or fifty-move rule
        if self.position.is_draw() {
            let score = Value::centi_pawn(0);

            self.notify_score_found(score, trace::ReturnKind::RuleDraw);

            return score;
        }

        // TODO: Do this better
        // Lookup if we've already searched this position
        let mut table_move = None;
        if let Some(entry) = self.trans_table.get(&self.position) {
            if entry.depth >= depth {
                match entry.bound {
                    Bound::Exact => {
                        self.notify_score_found(entry.score, trace::ReturnKind::TTExact);

                        return entry.score;
                    }
                    Bound::Lower => alpha = alpha.max(entry.score),
                    Bound::Upper => beta = beta.min(entry.score),
                }

                if beta <= alpha {
                    self.notify_score_found(entry.score, trace::ReturnKind::TTBound);

                    return entry.score;
                }
            }

            table_move = Some(entry.best_move);
        }

        let best_move = table_move.unwrap_or_else(|| moves[0]);
        // Ensures we do not enter quiescence search or prune the position when in check
        if !check && let Some(score) = self.prune(alpha, beta, depth, best_move, params) {
            return score;
        }

        self.reorder_moves(&mut moves, table_move);

        // Safety: `moves` is generated
        unsafe {
            self.search_moves(&moves, alpha, beta, depth, params, Self::search)
                .unwrap_or(Value::mate_in_ply_neg(0))
        }
    }

    fn aspiration_window_search(
        &mut self,
        alpha: Value,
        beta: Value,
        depth: i8,
        params: &mut SearchParams,
    ) -> Value {
        let entry_opt = self.trans_table.get(&self.position);
        if entry_opt.is_none() {
            return self.search(alpha, beta, depth, params);
        }
        let entry = entry_opt.unwrap();

        const START_DELTA: Value = Value::centi_pawn(15);

        // The low and high must stay within [alpha; beta] (inclusive),
        // but in case e.g. entry.score - START_DELTA is above beta,
        // we make sure the interval is not empty
        let mut low = alpha.max((entry.score - START_DELTA).min(beta - START_DELTA));
        let mut high = beta.min((entry.score + START_DELTA).max(alpha + START_DELTA));

        self.notify_aspiration_start(entry.score);

        for exp in 1.. {
            self.notify_aspiration_iter_start(low, high);

            let score = self.search(low, high, depth, params);
            let delta = START_DELTA * (1 << exp);

            if score >= high {
                if score >= beta {
                    self.notify_aspiration_iter_end(trace::AspirationResult::FailBeta);

                    return score;
                }

                self.notify_aspiration_iter_end(trace::AspirationResult::FailHigh);

                high = (score.max(entry.score) + delta).min(beta);
            } else if score <= low {
                // This should never happen when calling from the root
                if score <= alpha {
                    self.notify_aspiration_iter_end(trace::AspirationResult::FailAlpha);

                    return score;
                }

                self.notify_aspiration_iter_end(trace::AspirationResult::FailLow);

                low = (score.min(entry.score) - delta).max(alpha);
            } else {
                self.notify_aspiration_iter_end(trace::AspirationResult::InBounds);

                return score;
            }
        }

        unreachable!()
    }

    fn primary_variation(&mut self, depth: u8) -> Vec<Move> {
        let mut primary_variation = vec![];

        for _ in 0..depth {
            let entry = match self.trans_table.get(&self.position) {
                Some(entry) => entry,
                None => break,
            };

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
        let pv = self.primary_variation(depth);
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
                    Value::mate_in_ply_neg(0),
                    Value::mate_in_ply(0),
                    depth as i8,
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
