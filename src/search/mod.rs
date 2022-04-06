use std::mem::swap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::eval::Eval;
use crate::move_gen::MoveGen;
use crate::position::Position;
use crate::types::{Move, MoveKind, PieceKind, PseudoMove, Value};

use self::transposition_table::{Bound, Entry};

#[cfg(test)]
mod tests;
mod transposition_table;
pub use transposition_table::TranspositionTable;

struct SearchParams<'a> {
    nodes: u64,
    sel_depth: u8,
    start_depth: u8,
    stop_search: &'a AtomicBool,
    search_start: Instant,
}

pub struct Search<'client, 'f, E> {
    search_moves: Option<Vec<Move>>,
    search_depth: Option<u8>,
    search_nodes: Option<u64>,
    search_time: Option<Duration>,
    callbacks: Vec<Box<dyn FnMut(&SearchResult) + 'f>>,
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
        let callbacks = vec![];
        Self {
            search_moves: None,
            search_depth: None,
            search_nodes: None,
            search_time: None,
            callbacks,
            position,
            move_gen,
            eval,
            trans_table,
        }
    }

    pub fn moves(mut self, moves: &[PseudoMove]) -> Self {
        let legal_moves = self.move_gen.gen_all_moves(&self.position);
        let search_moves = moves
            .iter()
            .map(|&mv| mv.into_move(&legal_moves))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        self.search_moves = Some(search_moves);
        self
    }

    pub fn depth(mut self, depth: u8) -> Self {
        self.search_depth = Some(depth);
        self
    }

    pub fn time(mut self, time: Duration) -> Self {
        self.search_time = Some(time);
        self
    }

    pub fn nodes(mut self, nodes: u64) -> Self {
        self.search_nodes = Some(nodes);
        self
    }

    pub fn on_info<F: FnMut(&SearchResult) + 'f>(mut self, callback: F) -> Self {
        self.callbacks.push(Box::new(callback));
        self
    }

    fn search(
        &mut self,
        mut alpha: Value,
        mut beta: Value,
        depth_left: u8,
        params: &mut SearchParams,
    ) -> Value {
        if self.should_stop(&params) {
            return Value::from_inf(0);
        }

        let (mut moves, check) = self.move_gen.gen_all_moves_and_check(&self.position);

        if moves.len() == 0 {
            // Checkmate
            return if check {
                Value::from_neg_inf(((params.start_depth - depth_left + 1) / 2) as u16)
            // Stalemate
            } else {
                Value::from_cp(0)
            };
        }

        // Draw by threefold repetition or fifty-move rule
        if self.position.is_draw() {
            return Value::from_cp(0);
        }

        let orig_alpha = alpha;
        // TODO: Do this better
        let mut table_move = None;

        if let Some(entry) = self.trans_table.get(&self.position) {
            if entry.depth >= depth_left {
                match entry.bound {
                    Bound::Exact => return entry.score,
                    Bound::Lower => alpha = alpha.max(entry.score),
                    Bound::Upper => beta = beta.min(entry.score),
                }

                if alpha >= beta {
                    return entry.score;
                }
            }

            table_move = Some(entry.best_move);
        }

        let mut best_move = match table_move {
            Some(mv) => mv,
            None => moves[0],
        };
        let mut best_score = Value::from_neg_inf(0);

        if depth_left == 0 {
            return self.quiesce(
                alpha,
                beta,
                params.start_depth,
                &mut params.nodes,
                &mut params.sel_depth,
            );
        }

        self.reorder_moves(&mut moves, table_move);
        for mv in moves {
            let score;
            // Safety: Generated moves are guaranteed to be legal
            unsafe {
                self.position.make_move(mv);
                params.nodes += 1;
                score = -self.search(-beta, -alpha, depth_left - 1, params);
                self.position.unmake_move();
            }

            if score > best_score {
                best_score = score;
                best_move = mv;
                if score > alpha {
                    alpha = best_score;
                }
            }

            if alpha >= beta {
                let entry = Entry::new(alpha, best_move, Bound::Lower, depth_left);
                self.trans_table.insert(&self.position, entry);
                return alpha;
            }
        }

        let bound = if best_score < orig_alpha {
            Bound::Upper
        } else {
            Bound::Exact
        };
        let entry = Entry::new(best_score, best_move, bound, depth_left);
        self.trans_table.insert(&self.position, entry);
        best_score
    }

    fn reorder_moves(&mut self, moves: &mut [Move], best_move: Option<Move>) {
        let move_score = |mv: &Move| match mv.kind() {
            _ if Some(*mv) == best_move => 0,
            MoveKind::Regular => 4,
            MoveKind::Castling => 3,
            MoveKind::Promotion => match mv.promotion() {
                PieceKind::Queen => 2,
                _ => 5,
            },
            MoveKind::EnPassant => 1,
        };

        moves.sort_by_cached_key(move_score);
    }

    fn quiesce(
        &mut self,
        mut alpha: Value,
        beta: Value,
        start_depth: u8,
        nodes: &mut u64,
        sel_depth: &mut u8,
    ) -> Value {
        *sel_depth = start_depth.max(*sel_depth);

        // We assume that we can do at least as well as the static
        // eval of the current position, i.e. we don't consider zugzwang
        let static_eval = self.eval.eval(&self.position);
        if static_eval >= beta {
            return static_eval;
        } else if static_eval > alpha {
            alpha = static_eval;
        }

        let moves = self.move_gen.gen_captures(&self.position);
        for mv in moves {
            let score;
            // Safety: Generated moves are guaranteed to be legal
            unsafe {
                self.position.make_move(mv);
                *nodes += 1;
                score = -self.quiesce(-beta, -alpha, start_depth + 1, nodes, sel_depth);
                self.position.unmake_move();
            }

            if score >= beta {
                return score;
            }

            if score > alpha {
                alpha = score;
            }
        }

        alpha
    }

    fn aspire(
        &mut self,
        mut alpha: Value,
        mut beta: Value,
        depth: u8,
        params: &mut SearchParams,
    ) -> Value {
        if let Some(entry) = self.trans_table.get(&self.position) {
            //let mut iterations = 0;
            let entry = *entry;
            for exp in 0.. {
                let delta = Value::from_cp(5) * (1 << exp);
                alpha = alpha.min(entry.score - delta);
                beta = beta.max(entry.score + delta);

                let score = self.search(alpha, beta, depth, params);
                if score <= alpha {
                    alpha = score;
                } else if score >= beta {
                    beta = score;
                } else {
                    //dbg!(iterations);
                    return score;
                }

                //iterations += 1;
            }
        }

        self.search(alpha, beta, depth, params)
    }

    fn should_stop(&self, params: &SearchParams) -> bool {
        let time_searched = params.search_start.elapsed();
        params.stop_search.load(Ordering::Acquire)
            || self.search_time.map_or(false, |time| time_searched >= time)
            || self
                .search_nodes
                .map_or(false, |nodes| params.nodes >= nodes)
    }

    fn notify_info(&mut self, search_result: &SearchResult) {
        for callback in &mut self.callbacks {
            callback(search_result);
        }
    }

    pub fn start(mut self, stop_search: &AtomicBool) {
        let search_start = Instant::now();

        let mut search_moves = None;
        swap(&mut search_moves, &mut self.search_moves);
        let mut moves =
            search_moves.unwrap_or_else(|| self.move_gen.gen_all_moves(&self.position).into_vec());

        if moves.len() == 0 {
            return;
        }

        let max_depth = self.search_depth.unwrap_or(u8::MAX);

        for depth in 0..max_depth {
            let depth_start = Instant::now();

            let mut params = SearchParams {
                nodes: 0,
                sel_depth: depth,
                start_depth: depth + 1,
                stop_search,
                search_start,
            };

            let mut max_score = Value::from_neg_inf(0);
            let table_move = self
                .trans_table
                .get(&self.position)
                .map(|entry| entry.best_move);

            let mut best_move = match table_move {
                Some(mv) => mv,
                None => moves[0],
            };

            self.reorder_moves(&mut moves, table_move);
            for &mv in &moves {
                if self.should_stop(&params) {
                    return;
                }

                let score;
                unsafe {
                    self.position.make_move(mv);
                    params.nodes += 1;
                    score = -self.search(Value::from_neg_inf(0), -max_score, depth, &mut params);
                    self.position.unmake_move();
                }

                if score > max_score {
                    max_score = score;
                    best_move = mv;
                    let entry = Entry::new(max_score, best_move, Bound::Upper, depth + 1);
                    self.trans_table.insert(&self.position, entry);
                }
            }

            let entry = Entry::new(max_score, best_move, Bound::Exact, depth + 1);
            self.trans_table.insert(&self.position, entry);

            let mut primary_variation = vec![];
            while let Some(entry) = self.trans_table.get(&self.position) {
                primary_variation.push(entry.best_move);
                unsafe {
                    self.position.make_move(entry.best_move);
                }
            }

            for _ in &primary_variation {
                unsafe {
                    self.position.unmake_move();
                }
            }

            let hash_full = ((self.trans_table.len() * 1000) / self.trans_table.capacity()) as u32;
            let nps =
                (params.nodes as u128 * 1_000_000_000 / depth_start.elapsed().as_nanos()) as u64;
            let search_result = SearchResult {
                value: max_score,
                line: primary_variation,
                depth: depth + 1,
                sel_depth: params.sel_depth,
                nodes_searched: params.nodes,
                nps,
                total_duration: search_start.elapsed(),
                hash_full,
            };
            self.notify_info(&search_result);
        }
    }
}

#[derive(Clone)]
pub struct SearchResult {
    pub value: Value,
    pub line: Vec<Move>,
    pub depth: u8,
    pub sel_depth: u8,
    pub nodes_searched: u64,
    pub nps: u64,
    pub total_duration: Duration,
    pub hash_full: u32,
}
