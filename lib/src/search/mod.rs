//! Types and functions for searching for the best move in a position.
//!
//! The main way to start a search is to create a [`ThreadPool`] and give it a
//! [`SearchJob`] to run.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use itertools::Itertools;
use transposition_table::Bound;

use crate::collections::MoveList;
use crate::eval::{piece_value, Eval, StandardEval};
use crate::types::{value, IllegalMoveError, PseudoMove, Value};
use crate::MoveGen;
use crate::{types::Move, Position};

mod thread;
pub use thread::{info_channel, InfoReceiver, InfoSender, SearchInfo, ThreadPool};
#[cfg(test)]
mod tests;
mod transposition_table;
pub use transposition_table::{Entry, TranspositionTable};

/// The result of a search.
#[derive(Debug, Clone, Default)]
pub struct SearchResult {
    /// The evaluation of the position. `None` if the search was stopped before
    /// a result was found.
    pub evaluation: Option<SearchEvaluation>,
    /// Statistics about the search.
    pub stats: SearchStats,
}

/// A search job that can be run by a [`ThreadPool`].
#[derive(Clone)]
pub struct SearchJob<E = StandardEval> {
    position: Position,
    limits: Limits,
    move_gen: MoveGen,
    eval: E,
}

impl<E: Eval> SearchJob<E> {
    /// Creates a new search job builder given an evaluator. This function
    /// initializes the lookup tables, so it can be a slow operation.
    pub fn builder(eval: E) -> SearchJobBuilder<BuilderStateUninit, E> {
        SearchJobBuilder {
            limits: Limits::default(),
            state: BuilderStateUninit,
            move_gen: MoveGen::init(),
            eval,
        }
    }

    /// Starts a search to a given depth (without iterative deepening) and
    /// returns information about the pv and stats of the search. Panics if
    /// depth is not set.
    fn search(
        mut self,
        alpha: Value,
        beta: Value,
        search_start: Instant,
        kill_switch: Arc<AtomicBool>,
        t_table: Arc<TranspositionTable>,
    ) -> SearchResult {
        let depth = self.limits.depth.expect("depth should be set");
        let mut params = SearchParams {
            stats: SearchStats {
                sel_depth: depth,
                nodes: 0,
            },
            search_start,
            kill_switch,
            t_table,
            start_depth: depth,
        };

        let mut moves = self
            .limits
            .moves
            .clone()
            .unwrap_or_else(|| self.move_gen.gen_all_moves(&self.position).to_vec());
        self.reorder_moves(
            &mut moves,
            params.t_table.get(&self.position).map(|e| e.best_move),
        );

        let score = self.search_moves(&moves, depth, alpha, beta, &mut params);
        if let Some(score) = score {
            let pv = self.primary_variation(depth, &params.t_table);
            let result = SearchEvaluation { score, pv };
            SearchResult {
                evaluation: Some(result),
                stats: params.stats,
            }
        } else {
            SearchResult {
                evaluation: None,
                stats: params.stats,
            }
        }
    }

    fn alpha_beta(
        &mut self,
        depth: i8,
        mut alpha: Value,
        mut beta: Value,
        params: &mut SearchParams,
    ) -> Option<Value> {
        if self.should_stop(params) {
            return None;
        }

        let (mut moves, check) = self.gen_moves_and_check();

        if moves.is_empty() {
            // Checkmate
            return if check {
                Some(Value::neg_mate_in_ply(0))
            // Stalemate
            } else {
                Some(Value::centipawn(0))
            };
        }

        // Draw by threefold repetition or fifty-move rule
        if self.position.is_rule_draw() {
            return Some(Value::centipawn(0));
        }

        if let Some(entry) = params.t_table.get(&self.position) {
            if entry.depth >= depth {
                match entry.bound {
                    Bound::Exact => return Some(entry.score),
                    Bound::Lower => alpha = alpha.max(entry.score),
                    Bound::Upper => beta = beta.min(entry.score),
                }

                if alpha >= beta {
                    return Some(entry.score);
                }
            }
            self.reorder_moves(&mut moves, Some(entry.best_move));
        }

        if !check && depth <= 0 {
            return self.quiesce(alpha, beta, params.start_depth - depth, params);
        }

        self.search_moves(&moves, depth, alpha, beta, params)
    }

    #[inline(always)]
    fn search_moves(
        &mut self,
        moves: &[Move],
        depth: i8,
        alpha: Value,
        beta: Value,
        params: &mut SearchParams,
    ) -> Option<Value> {
        let mut best_move = *moves.first()?;
        let mut best_score = value::NEG_INF;
        let mut low = alpha;

        for &mv in moves {
            self.position.make_move(mv);
            params.stats.nodes += 1;
            let res = self.alpha_beta(depth - 1, -beta.dec_mate(), -low.dec_mate(), params);
            self.position.unmake_move();
            let score = -res?.inc_mate();

            if score >= beta {
                let entry = Entry::new(score, mv, Bound::Lower, depth);
                params.t_table.insert(&self.position, entry);
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
        params.t_table.insert(&self.position, entry);

        Some(best_score)
    }

    fn should_stop(&self, params: &SearchParams) -> bool {
        // Only consider stopping every 2^11 = 2048 nodes
        if params.stats.nodes & ((1 << 11) - 1) != 0 {
            return false;
        }
        params.kill_switch.load(Ordering::Relaxed)
            || self.limits.nodes.is_some_and(|n| params.stats.nodes >= n)
            || self
                .limits
                .time
                .is_some_and(|t| params.search_start.elapsed() >= t)
    }

    fn quiesce(
        &mut self,
        mut alpha: Value,
        beta: Value,
        sel_depth: i8,
        params: &mut SearchParams,
    ) -> Option<Value> {
        if self.should_stop(params) {
            return None;
        }

        params.stats.sel_depth = sel_depth.max(params.stats.sel_depth);

        // We assume that we can do at least as well as the static
        // eval of the current position, i.e. we don't consider zugzwang
        let static_eval = self.static_eval();
        if static_eval >= beta {
            return Some(static_eval);
        } else if static_eval > alpha {
            alpha = static_eval;
        }

        let mut best_score = static_eval;

        let mut moves = self.move_gen.gen_captures(&self.position);
        self.reorder_moves(&mut moves, None);
        for mv in moves {
            self.position.make_move(mv);
            params.stats.nodes += 1;
            let res = self.quiesce(-beta, -alpha, sel_depth + 1, params);
            self.position.unmake_move();
            let score = -res?;

            if score >= beta {
                return Some(score);
            }

            if score > best_score {
                best_score = score;
                if score > alpha {
                    alpha = score;
                }
            }
        }

        Some(best_score)
    }

    fn reorder_moves(&self, mut moves: &mut [Move], best_move: Option<Move>) {
        if let Some(best_move) = best_move {
            if let Some(i) = moves.iter().position(|&mv| mv == best_move) {
                moves.swap(0, i);
                moves = &mut moves[1..];
            } else {
                log::warn!("ttable move not found in move list");
            }
        }

        // MVV-LVA ordering
        moves.sort_by_key(|mv| {
            if let Some(victim) = self.position.pieces.get(mv.to()) {
                -piece_value(victim.kind())
            } else {
                0
            }
        });
    }

    fn gen_moves_and_check(&self) -> (MoveList, bool) {
        self.move_gen.gen_all_moves_and_check(&self.position)
    }

    fn static_eval(&self) -> Value {
        self.eval.eval(&self.position)
    }

    fn primary_variation(&mut self, depth: i8, t_table: &TranspositionTable) -> Vec<Move> {
        let mut primary_variation = vec![];

        for _ in 0..depth {
            let entry = match t_table.get(&self.position) {
                Some(entry) => entry,
                None => break,
            };

            // Sanity checking in case of hash collision
            let moves = self.move_gen.gen_all_moves(&self.position);
            if !moves.contains(entry.best_move) {
                log::warn!(
                    "hash collision detected, move: {}, position:\n{}",
                    entry.best_move,
                    self.position
                );
                break;
            }

            primary_variation.push(entry.best_move);
            self.position.make_move(entry.best_move);
        }

        for _ in &primary_variation {
            self.position.unmake_move();
        }

        primary_variation
    }
}

impl SearchJob {
    /// Creates a new default search job builder by initializing the lookup
    /// tables. This can therfore be a slow operation.
    pub fn default_builder() -> SearchJobBuilder<BuilderStateUninit> {
        SearchJobBuilder {
            limits: Limits::default(),
            state: BuilderStateUninit,
            move_gen: MoveGen::init(),
            eval: StandardEval,
        }
    }
}

#[derive(Default, Clone, Debug)]
struct Limits {
    moves: Option<Vec<Move>>,
    depth: Option<i8>,
    nodes: Option<u64>,
    time: Option<Duration>,
    allow_early_stop: bool,
}

/// The evaluation according to a search, including the score and principal
/// variation.
#[derive(Debug, Clone)]
pub struct SearchEvaluation {
    /// The score given to the position.
    pub score: Value,
    /// The principal variation of the search.
    pub pv: Vec<Move>,
}

/// Statistics about a search, including the depth and number of nodes searched.
#[derive(Debug, Clone, Copy, Default)]
pub struct SearchStats {
    /// The maximum depth reached in the search.
    pub sel_depth: i8,
    /// The number of nodes searched.
    pub nodes: u64,
}

impl SearchStats {
    pub fn combine(self, other: Self) -> Self {
        Self {
            sel_depth: self.sel_depth.max(other.sel_depth),
            nodes: self.nodes + other.nodes,
        }
    }
}

struct SearchParams {
    stats: SearchStats,
    search_start: Instant,
    kill_switch: Arc<AtomicBool>,
    t_table: Arc<TranspositionTable>,
    start_depth: i8,
}

/// A builder for a [`SearchJob`].
pub struct SearchJobBuilder<S, E = StandardEval> {
    limits: Limits,
    state: S,
    move_gen: MoveGen,
    eval: E,
}

impl<E> SearchJobBuilder<BuilderStateUninit, E> {
    /// Sets the position to search from.
    pub fn position(self, position: Position) -> SearchJobBuilder<BuilderStateInit, E> {
        SearchJobBuilder {
            limits: self.limits,
            state: BuilderStateInit { position },
            move_gen: self.move_gen,
            eval: self.eval,
        }
    }
}

impl<E> SearchJobBuilder<BuilderStateInit, E> {
    /// Search only the given moves.
    pub fn moves(
        mut self,
        moves: impl IntoIterator<Item = PseudoMove>,
    ) -> Result<Self, IllegalMoveError> {
        let legal_moves = self.move_gen.gen_all_moves(&self.state.position);
        self.limits.moves = Some(
            moves
                .into_iter()
                .map(|mv| mv.into_move(&legal_moves))
                .try_collect()?,
        );
        Ok(self)
    }

    /// Sets the maximum depth to search to.
    pub fn depth(mut self, depth: i8) -> Self {
        self.limits.depth = Some(depth);
        self
    }

    /// Sets the maximum number of nodes to search to.
    pub fn nodes(mut self, nodes: u64) -> Self {
        self.limits.nodes = Some(nodes);
        self
    }

    /// Sets the maximum time to search for.
    pub fn time(mut self, time: Duration) -> Self {
        self.limits.time = Some(time);
        self
    }

    /// Sets whether to allow the search to stop early.
    pub fn allow_early_stop(mut self, allow: bool) -> Self {
        self.limits.allow_early_stop = allow;
        self
    }

    /// Builds the search job.
    pub fn build(self) -> SearchJob<E> {
        SearchJob {
            position: self.state.position,
            limits: self.limits,
            move_gen: self.move_gen,
            eval: self.eval,
        }
    }
}

/// A state indicating that the position has not been set yet.
pub struct BuilderStateUninit;
/// A state indicating that the position has been set.
pub struct BuilderStateInit {
    position: Position,
}
