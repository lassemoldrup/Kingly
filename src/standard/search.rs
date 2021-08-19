use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use std::mem::swap;

use crate::framework::{Eval, MoveGen};
use crate::framework::search::SearchResult;
use crate::framework::value::Value;
use crate::standard::Position;
use crate::framework::moves::{PseudoMove, Move};

pub struct Search<'client, MG, E> {
    search_moves: Option<Vec<Move>>,
    search_depth: Option<u32>,
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

    fn on_info<F: FnMut(&SearchResult) + 'client>(mut self, callback: F) -> Self
    {
        self.callbacks.push(Box::new(callback));
        self
    }

    fn start(mut self, stop_search: &AtomicBool) {
        let mut search_moves = None;
        swap(&mut search_moves, &mut self.search_moves);
        let moves = search_moves
            .unwrap_or_else(|| self.move_gen.gen_all_moves(&self.position).into_vec());

        for depth in 0.. {
            let start = Instant::now();

            let mut nodes = 0;
            let mut max_score = Value::NegInf;
            let mut best_line = vec![];

            for &mv in &moves {
                if stop_search.load(Ordering::Acquire) {
                    return;
                }

                let score;
                unsafe {
                    self.position.make_move(mv);
                    score = self.alpha_beta(Value::NegInf, Value::Inf, depth, &mut nodes);
                    self.position.unmake_move();
                }

                if score > max_score {
                    max_score = score;
                    best_line = vec![mv];
                }
            }

            let duration = start.elapsed();
            let search_result = SearchResult::new(max_score, best_line, depth + 1, nodes, duration);
            for callback in &mut self.callbacks {
                callback(&search_result);
            }
        }
    }
}