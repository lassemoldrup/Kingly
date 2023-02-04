use std::fmt::{self, Display, Formatter};

use crate::types::{Move, Value};

use super::{Search, SearchThread};

#[derive(Clone, Copy, Debug)]
pub enum ReturnKind {
    Best(Move),
    Beta(Move),
    TTExact,
    TTBound,
    Quiesce,
    NullMove,
    Checkmate,
    Stalemate,
    /// Threefold repetition or fifty-move rule
    RuleDraw,
}

impl Display for ReturnKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ReturnKind::Best(mv) => write!(f, "Best: {mv}"),
            ReturnKind::Beta(mv) => write!(f, "Beta: {mv}"),
            ReturnKind::TTExact => write!(f, "T.T. Exact"),
            ReturnKind::TTBound => write!(f, "T.T. Bound"),
            ReturnKind::Quiesce => write!(f, "Quiscence"),
            ReturnKind::NullMove => write!(f, "Null Move"),
            ReturnKind::Checkmate => write!(f, "C. Mate"),
            ReturnKind::Stalemate => write!(f, "S. Mate"),
            ReturnKind::RuleDraw => write!(f, "Draw"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum AspirationResult {
    FailHigh,
    FailBeta,
    FailLow,
    FailAlpha,
    InBounds,
}

impl Display for AspirationResult {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            AspirationResult::FailHigh => write!(f, "High"),
            AspirationResult::FailBeta => write!(f, "Beta"),
            AspirationResult::FailLow => write!(f, "Low"),
            AspirationResult::FailAlpha => write!(f, "Alpha"),
            AspirationResult::InBounds => write!(f, "In"),
        }
    }
}

pub trait Observer {
    fn new_depth(&mut self, depth: u8);
    fn move_made(&mut self, mv: Move, alpha: Value, beta: Value);
    fn move_unmade(&mut self, mv: Move);
    fn score_found(&mut self, score: Value, kind: ReturnKind);
    fn aspiration_start(&mut self, prev: Value);
    fn aspiration_iter_start(&mut self, low: Value, high: Value);
    fn aspiration_iter_end(&mut self, result: AspirationResult);
}

impl Observer for () {
    fn new_depth(&mut self, _depth: u8) {}
    fn move_made(&mut self, _mv: Move, _alpha: Value, _beta: Value) {}
    fn move_unmade(&mut self, _mv: Move) {}
    fn score_found(&mut self, _score: Value, _kind: ReturnKind) {}
    fn aspiration_start(&mut self, _prev: Value) {}
    fn aspiration_iter_start(&mut self, _low: Value, _high: Value) {}
    fn aspiration_iter_end(&mut self, _result: AspirationResult) {}
}

impl<'c, 'f, E> Search<'c, 'f, E> {
    pub fn register<O>(self, observer: O) -> Search<'c, 'f, E, O>
    where
        O: Observer,
    {
        Search {
            limits: self.limits,
            num_threads: self.num_threads,
            callbacks: self.callbacks,
            position: self.position,
            move_gen: self.move_gen,
            eval: self.eval,
            trans_table: self.trans_table,
            observer,
        }
    }
}

impl<'c, 's, E, O: Observer> SearchThread<'c, 's, E, O> {
    pub(super) fn notify_new_depth(&mut self, depth: u8) {
        self.observer.new_depth(depth);
    }

    pub(super) fn notify_move_made(&mut self, mv: Move, alpha: Value, beta: Value) {
        self.observer.move_made(mv, alpha, beta);
    }

    pub(super) fn notify_move_unmade(&mut self, mv: Move) {
        self.observer.move_unmade(mv);
    }

    pub(super) fn notify_score_found(&mut self, score: Value, kind: ReturnKind) {
        self.observer.score_found(score, kind);
    }

    pub(super) fn notify_aspiration_start(&mut self, prev: Value) {
        self.observer.aspiration_start(prev);
    }

    pub(super) fn notify_aspiration_iter_start(&mut self, low: Value, high: Value) {
        self.observer.aspiration_iter_start(low, high);
    }

    pub(super) fn notify_aspiration_iter_end(&mut self, result: AspirationResult) {
        self.observer.aspiration_iter_end(result);
    }
}
