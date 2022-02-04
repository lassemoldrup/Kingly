use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, Duration};
use std::mem::swap;

use crate::framework::{Eval, MoveGen};
use crate::framework::search::SearchResult;
use crate::framework::value::Value;
use crate::standard::Position;
use crate::framework::moves::{PseudoMove, Move};

#[cfg(test)]
mod tests;

pub struct Search<'client, MG, E> {
    search_moves: Option<Vec<Move>>,
    search_depth: Option<u32>,
    search_nodes: Option<u64>,
    search_time: Option<Duration>,
    callbacks: Vec<Box<dyn FnMut(&SearchResult) + 'client>>,
    position: Position,
    move_gen: &'client MG,
    eval: &'client E,
}

impl<'client, MG, E> Search<'client, MG, E> where
    MG: MoveGen<Position>,
    E: Eval<Position>
{
    pub fn new(position: Position, move_gen: &'client MG, eval: &'client E) -> Self {
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
        }
    }

    fn alpha_beta(&mut self, mut alpha: Value, beta: Value, depth: u32, nodes: &mut u64) -> Value {
        if depth == 0 {
            *nodes += 1;
            return self.eval.eval(&self.position);
        }

        let moves = self.move_gen.gen_all_moves(&self.position);
        for mv in moves {
            let score;
            unsafe {
                self.position.make_move(mv);
                score = -Self::alpha_beta(self, -beta, -alpha, depth - 1, nodes);
                self.position.unmake_move();
            }

            if score >= beta  {
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
        let moves = search_moves
            .unwrap_or_else(|| self.move_gen.gen_all_moves(&self.position).into_vec());

        let max_depth = self.search_depth
            .unwrap_or(u32::MAX);

        for depth in 0..max_depth {
            let depth_start = Instant::now();

            let mut nodes = 0;
            let mut max_score = Value::NegInf;
            let mut primary_variation = vec![];

            for &mv in &moves {
                if self.should_stop(stop_search, search_start.elapsed(), nodes) {
                    return;
                }

                let score;
                unsafe {
                    self.position.make_move(mv);
                    score = -self.alpha_beta(Value::NegInf, Value::Inf, depth, &mut nodes);
                    self.position.unmake_move();
                }

                if score > max_score {
                    max_score = score;
                    primary_variation = vec![mv];
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