use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, Duration};
use std::mem::swap;

use crate::framework::piece::PieceKind;
use crate::framework::{Eval, MoveGen, Position as _};
use crate::framework::search::SearchResult;
use crate::framework::value::Value;
use crate::framework::moves::{PseudoMove, Move};
use crate::standard::Position;

use self::transposition_table::{TranspositionTable, Bound, Entry};

#[cfg(test)]
mod tests;
pub mod transposition_table;

pub struct Search<'client, 'f, MG, E> {
    search_moves: Option<Vec<Move>>,
    search_depth: Option<u32>,
    search_nodes: Option<u64>,
    search_time: Option<Duration>,
    callbacks: Vec<Box<dyn FnMut(&SearchResult) + 'f>>,
    position: Position,
    move_gen: &'client MG,
    eval: &'client E,
    trans_table: &'client mut TranspositionTable,
}

impl<'client, 'f, MG, E> Search<'client, 'f, MG, E> where
    MG: MoveGen<Position>,
    E: Eval<Position>
{
    pub fn new(position: Position, move_gen: &'client MG, eval: &'client E, trans_table: &'client mut TranspositionTable) -> Self
    {
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

    fn alpha_beta(&mut self, mut alpha: Value, beta: Value, depth: u32, start_depth: u32, nodes: &mut u64, sel_depth: &mut u32) -> Value {
        let (mut moves, check) = self.move_gen.gen_all_moves_and_check(&self.position);
        
        if moves.len() == 0 {
            *nodes += 1;
            // Checkmate
            return if check {
                Value::NegInf((start_depth - depth + 1) / 2)
            // Stalemate
            } else {
                Value::CentiPawn(0)
            };
        }
        
        // Draw by threefold repetition or fifty-move rule
        if self.position.is_draw() {
            return Value::CentiPawn(0);
        }

        // TODO: Do this better
        let mut table_move = None;

        if let Some(entry) = self.trans_table.get(&self.position) {
            if entry.depth >= depth {
                match entry.bound {
                    Bound::Exact => return entry.score,
                    Bound::Lower => if entry.score <= alpha {
                        return alpha;
                    }
                    Bound::Upper => if entry.score >= beta {
                        return beta;
                    },
                }
            }
            table_move = Some(entry.best_move);
        }

        let mut best_move = match table_move {
            Some(mv) => mv,
            None => moves[0],
        };

        if depth == 0 {
            return self.quiesce(alpha, beta, start_depth, nodes, sel_depth);
        }

        self.reorder_moves(&mut moves, table_move);
        for mv in moves {
            let score;
            // Safety: Generated moves are guaranteed to be legal
            unsafe {
                self.position.make_move(mv);
                *nodes += 1;
                score = -self.alpha_beta(-beta, -alpha, depth - 1, start_depth, nodes, sel_depth);
                self.position.unmake_move();
            }

            if score >= beta {
                let entry = Entry::new(beta, best_move, Bound::Upper, depth);
                self.trans_table.insert(&self.position, entry);
                return beta;
            }

            if score > alpha {
                alpha = score;
                best_move = mv;
                let entry = Entry::new(alpha, best_move, Bound::Lower, depth);
                self.trans_table.insert(&self.position, entry);
            }
        }

        let entry = Entry::new(alpha, best_move, Bound::Exact, depth);
        self.trans_table.insert(&self.position, entry);
        alpha
    }

    fn reorder_moves(&mut self, moves: &mut [Move], best_move: Option<Move>) {
        let move_score = |mv: &Move| match mv {
            _ if Some(*mv) == best_move => 0,
            Move::Regular(_, _) => 4,
            Move::Castling(_, _) => 3,
            Move::Promotion(_, _, kind) => match kind {
                PieceKind::Queen => 2,
                _ => 5,
            },
            Move::EnPassant(_, _) => 1,
        };

        moves.sort_by_cached_key(move_score);
    }

    fn quiesce(&mut self, mut alpha: Value, beta: Value, start_depth: u32, nodes: &mut u64, sel_depth: &mut u32) -> Value {
        *sel_depth = start_depth.max(*sel_depth);
        
        // We assume that we can do at least as well as the static
        // eval of the current position, i.e. we don't consider zugzwang 
        let static_eval = self.eval.eval(&self.position);
        if static_eval >= beta {
            return beta;
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
                return beta;
            }

            if score > alpha {
                alpha = score;
            }
        }

        alpha
    }

    fn should_stop(&self, stop_search: &AtomicBool, time_searched: Duration, nodes_searched: u64) -> bool {
        stop_search.load(Ordering::Acquire)
            || self.search_time.map_or(false, |time| time_searched >= time)
            || self.search_nodes.map_or(false, |nodes| nodes_searched >= nodes)
    }

    fn notify_info(&mut self, search_result: &SearchResult) {
        for callback in &mut self.callbacks {
            callback(search_result);
        }
    }
}

impl<'client, 'f, MG, E>  crate::framework::search::Search<'f> for Search<'client, 'f, MG, E> where
    MG: MoveGen<Position>,
    E: Eval<Position>
{
    fn moves(mut self, moves: &[PseudoMove]) -> Self {
        let legal_moves = self.move_gen.gen_all_moves(&self.position);
        let search_moves = moves.iter()
            .map(|&mv| mv.into_move(&legal_moves))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        self.search_moves = Some(search_moves);
        self
    }

    fn depth(mut self, depth: u32) -> Self {
        self.search_depth = Some(depth);
        self
    }

    fn time(mut self, time: Duration) -> Self {
        self.search_time = Some(time);
        self
    }

    fn nodes(mut self, nodes: u64) -> Self {
        self.search_nodes = Some(nodes);
        self
    }

    fn on_info<F: FnMut(&SearchResult) + 'f>(mut self, callback: F) -> Self
    {
        self.callbacks.push(Box::new(callback));
        self
    }

    fn start(mut self, stop_search: &AtomicBool) {
        let search_start = Instant::now();

        let mut search_moves = None;
        swap(&mut search_moves, &mut self.search_moves);
        let mut moves = search_moves
            .unwrap_or_else(|| self.move_gen.gen_all_moves(&self.position).into_vec());

        if moves.len() == 0 {
            return;
        }

        let max_depth = self.search_depth
            .unwrap_or(u32::MAX);

        for depth in 0..max_depth {
            let depth_start = Instant::now();

            let mut nodes = 0;
            let mut sel_depth = depth;
            let mut max_score = Value::NegInf(0);
            let table_move = self.trans_table.get(&self.position)
                .map(|entry| entry.best_move);

            let mut best_move = match table_move {
                Some(mv) => mv,
                None => moves[0],
            };

            self.reorder_moves(&mut moves, table_move);
            for &mv in &moves {
                if self.should_stop(stop_search, search_start.elapsed(), nodes) {
                    return;
                }

                let score;
                unsafe {
                    self.position.make_move(mv);
                    nodes += 1;
                    score = -self.alpha_beta(Value::NegInf(0), -max_score, depth, depth + 1, &mut nodes, &mut sel_depth);
                    self.position.unmake_move();
                }

                if score > max_score {
                    max_score = score;
                    best_move = mv;
                    let entry = Entry::new(max_score, best_move, Bound::Lower, depth + 1);
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

            let hash_full = (self.trans_table.len() * 1000) / self.trans_table.capacity();
            let duration = depth_start.elapsed();
            let search_result = SearchResult::new(
                max_score,
                primary_variation,
                depth + 1,
                sel_depth,
                nodes,
                duration,
                hash_full as u32,
            );
            self.notify_info(&search_result);
        }
    }
}