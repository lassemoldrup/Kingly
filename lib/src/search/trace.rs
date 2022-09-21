use std::cell::RefCell;
use std::rc::Weak;

use crate::types::{Move, Value};

use super::Search;

#[derive(Clone, Copy, Debug)]
pub enum ReturnKind {
    Best(Move),
    Beta(Move),
    TTExact,
    TTBound,
    Quiesce,
    Checkmate,
    Stalemate,
    // Threefold repetition or fifty-move rule
    RuleDraw,
}

#[derive(Clone, Copy, Debug)]
pub enum AspirationResult {
    FailHigh,
    FailBeta,
    FailLow,
    FailAlpha,
    InBounds,
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

impl<'c, 'f, E> Search<'c, 'f, E> {
    pub fn register<O>(mut self, observer: Weak<RefCell<O>>) -> Self
    where
        O: Observer + 'static,
    {
        self.observers.push(observer as Weak<RefCell<dyn Observer>>);
        self
    }

    fn notify_observers(&self, f: impl Fn(&mut dyn Observer)) {
        for observer in self.observers.iter().filter_map(|o| o.upgrade()) {
            f(&mut *(*observer).borrow_mut());
        }
    }

    pub(super) fn notify_new_depth(&self, depth: u8) {
        self.notify_observers(|o| o.new_depth(depth));
    }

    pub(super) fn notify_move_made(&self, mv: Move, alpha: Value, beta: Value) {
        self.notify_observers(|o| o.move_made(mv, alpha, beta));
    }

    pub(super) fn notify_move_unmade(&self, mv: Move) {
        self.notify_observers(|o| o.move_unmade(mv));
    }

    pub(super) fn notify_score_found(&self, score: Value, kind: ReturnKind) {
        self.notify_observers(|o| o.score_found(score, kind));
    }

    pub(super) fn notify_aspiration_start(&self, prev: Value) {
        self.notify_observers(|o| o.aspiration_start(prev));
    }

    pub(super) fn notify_aspiration_iter_start(&self, low: Value, high: Value) {
        self.notify_observers(|o| o.aspiration_iter_start(low, high));
    }

    pub(super) fn notify_aspiration_iter_end(&self, result: AspirationResult) {
        self.notify_observers(|o| o.aspiration_iter_end(result));
    }
}
