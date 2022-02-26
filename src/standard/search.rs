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
mod transposition_table;

pub struct Search<'client, MG, E> {
    search_moves: Option<Vec<Move>>,
    search_depth: Option<u32>,
    search_nodes: Option<u64>,
    search_time: Option<Duration>,
    callbacks: Vec<Box<dyn FnMut(&SearchResult) + 'client>>,
    position: Position,
    move_gen: &'client MG,
    eval: &'client E,
    transposition_table: TranspositionTable,
}

impl<'client, MG, E> Search<'client, MG, E> where
    MG: MoveGen<Position>,
    E: Eval<Position>
{
    pub fn new(position: Position, move_gen: &'client MG, eval: &'client E) -> Self {
        let callbacks = vec![];
        let transposition_table = TranspositionTable::new();
        Self {
            search_moves: None,
            search_depth: None,
            search_nodes: None,
            search_time: None,
            callbacks,
            position,
            move_gen,
            eval,
            transposition_table,
        }
    }

    fn alpha_beta(&mut self, mut alpha: Value, beta: Value, depth: u32, start_depth: u32, nodes: &mut u64) -> Value {
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

        let mut best_move = moves[0];

        if let Some(entry) = self.transposition_table.get(&self.position) {
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
            best_move = entry.best_move;
        }

        if depth == 0 {
            *nodes += 1;
            return self.eval.eval(&self.position);
        }

        self.reorder_moves(&mut moves, best_move);
        for mv in moves {
            let score;
            // Safety: Generated moves are guaranteed to be legal
            unsafe {
                self.position.make_move(mv);
                score = -self.alpha_beta(-beta, -alpha, depth - 1, start_depth, nodes);
                self.position.unmake_move();
            }

            if score >= beta {
                let entry = Entry::new(beta, best_move, Bound::Upper, depth);
                self.transposition_table.insert(&self.position, entry);
                return beta;
            }

            if score > alpha {
                alpha = score;
                best_move = mv;
                let entry = Entry::new(alpha, best_move, Bound::Lower, depth);
                self.transposition_table.insert(&self.position, entry);
            }
        }

        let entry = Entry::new(alpha, best_move, Bound::Exact, depth);
        self.transposition_table.insert(&self.position, entry);
        alpha
    }

    fn reorder_moves(&mut self, moves: &mut [Move], best_move: Move) {
        let move_score = |mv: &Move| match mv {
            _ if *mv == best_move => 0,
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

impl<'client, MG, E>  crate::framework::search::Search<'client> for Search<'client, MG, E> where
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

    fn on_info<F: FnMut(&SearchResult) + 'client>(mut self, callback: F) -> Self
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

        let max_depth = self.search_depth
            .unwrap_or(u32::MAX);

        for depth in 0..max_depth {
            let depth_start = Instant::now();

            let mut nodes = 0;
            let mut max_score = Value::NegInf(0);
            let mut best_move = moves[0];

            if let Some(entry) = self.transposition_table.get(&self.position) {
                best_move = entry.best_move;
            }

            self.reorder_moves(&mut moves, best_move);
            for &mv in &moves {
                if self.should_stop(stop_search, search_start.elapsed(), nodes) {
                    return;
                }

                let score;
                unsafe {
                    self.position.make_move(mv);
                    score = -self.alpha_beta(Value::NegInf(0), -max_score, depth, depth + 1, &mut nodes);
                    self.position.unmake_move();
                }

                if score > max_score {
                    max_score = score;
                    best_move = mv;
                    let entry = Entry::new(max_score, best_move, Bound::Lower, depth + 1);
                    self.transposition_table.insert(&self.position, entry);
                }
            }

            let entry = Entry::new(max_score, best_move, Bound::Exact, depth + 1);
            self.transposition_table.insert(&self.position, entry);

            let mut primary_variation = vec![];
            while let Some(entry) = self.transposition_table.get(&self.position) {
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

            let duration = depth_start.elapsed();
            let search_result = SearchResult::new(
                max_score,
                primary_variation,
                depth + 1,
                nodes,
                duration
            );
            self.notify_info(&search_result);
        }
    }
}