use crate::framework::search::SearchResult;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::framework::{MoveGen, Eval};
use crate::framework::value::Value;
use crate::standard::Position;
use std::time::Instant;

pub struct Search<'a, MG, E> {
    callbacks: Vec<Box<dyn FnMut(&SearchResult) + 'a>>,
    position: Position,
    move_gen: &'a MG,
    eval: &'a E,
}

impl<'a, MG, E> Search<'a, MG, E> where
    MG: MoveGen<Position>,
    E: Eval<Position>
{
    pub fn new(position: Position, move_gen: &'a MG, eval: &'a E) -> Self {
        let callbacks = vec![];
        Self {
            callbacks, position, move_gen, eval
        }
    }

    fn alpha_beta(&mut self, mut alpha: Value, beta: Value, depth: u32) -> Value {
        if depth == 0 {
            return self.eval.eval(&self.position)
        }

        let moves = self.move_gen.gen_all_moves(&self.position);
        for mv in moves {
            let score;
            unsafe {
                self.position.make_move(mv);
                score = -Self::alpha_beta(self, -beta, -alpha, depth - 1);
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

impl<'a, MG, E>  crate::framework::search::Search<'a> for Search<'a, MG, E> where
    MG: MoveGen<Position>,
    E: Eval<Position>
{
    fn on_info<F: FnMut(&SearchResult) + 'a>(&mut self, callback: F) {
        self.callbacks.push(Box::new(callback));
    }

    fn start(mut self, stop_switch: Arc<AtomicBool>) {
        let moves = self.move_gen.gen_all_moves(&self.position);
        for depth in 0.. {
            let start = Instant::now();

            let mut nodes = 0;
            let mut max_score = Value::NegInf;
            let mut best_line = vec![];

            for &mv in &moves {
                if stop_switch.load(Ordering::Relaxed) {
                    return;
                }

                let score;
                unsafe {
                    self.position.make_move(mv);
                    score = self.alpha_beta(Value::NegInf, Value::Inf, depth);
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