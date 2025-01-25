use std::fmt::{self, Display, Formatter};
use std::sync::{Arc, Mutex};

use crate::types::{Move, Value};

use super::{NodeType, SearchJob};

impl<E, O: SearchObserver> SearchJob<E, O> {
    pub(super) fn on_node_enter<N: NodeType>(
        &mut self,
        alpha: Value,
        beta: Value,
        mv: Move,
        pvs_re_search: bool,
    ) {
        self.observer
            .on_node_enter::<N>(self.worker_id, alpha, beta, Some(mv), pvs_re_search);
    }

    pub(super) fn on_node_exit<N: NodeType>(
        &mut self,
        mv: Move,
        res: Option<(Value, O::ReturnKind)>,
    ) {
        let (score, ret) = res.unzip();
        self.observer
            .on_node_exit::<N>(self.worker_id, Some(mv), ret.into(), score);
    }
}

pub trait SearchObserver {
    type ReturnKind: ReturnKindTrait + Clone;

    fn on_depth(&mut self, _depth: i8) {}
    fn on_aspiration_window(&mut self, _alpha: Value, _beta: Value) {}
    fn on_node_enter<N: NodeType>(
        &mut self,
        _worker_id: usize,
        _alpha: Value,
        _beta: Value,
        _mv: Option<Move>,
        _pvs_re_search: bool,
    ) {
    }
    fn on_node_exit<N: NodeType>(
        &mut self,
        _worker_id: usize,
        _mv: Option<Move>,
        _ret: Self::ReturnKind,
        _score: Option<Value>,
    ) {
    }
}

impl<T: SearchObserver> SearchObserver for Arc<Mutex<T>> {
    type ReturnKind = T::ReturnKind;

    fn on_depth(&mut self, depth: i8) {
        self.lock().unwrap().on_depth(depth)
    }

    fn on_aspiration_window(&mut self, alpha: Value, beta: Value) {
        self.lock().unwrap().on_aspiration_window(alpha, beta)
    }

    fn on_node_enter<N: NodeType>(
        &mut self,
        worker_id: usize,
        alpha: Value,
        beta: Value,
        mv: Option<Move>,
        pvs_re_search: bool,
    ) {
        self.lock()
            .unwrap()
            .on_node_enter::<N>(worker_id, alpha, beta, mv, pvs_re_search)
    }

    fn on_node_exit<N: NodeType>(
        &mut self,
        worker_id: usize,
        mv: Option<Move>,
        ret: Self::ReturnKind,
        score: Option<Value>,
    ) {
        self.lock()
            .unwrap()
            .on_node_exit::<N>(worker_id, mv, ret, score)
    }
}

#[derive(Clone, Copy)]
pub struct EmptyObserver;
impl SearchObserver for EmptyObserver {
    type ReturnKind = EmptyReturnKind;
}

pub trait ReturnKindTrait: From<ReturnKind> + From<Option<Self>> {}

#[derive(Clone)]
pub struct EmptyReturnKind;

#[derive(Clone)]
pub enum ReturnKind {
    Pv(Move),
    FailHigh(Move),
    FailLow(Move),
    TTExact(Move),
    TTUpper(Move),
    TTLower(Move),
    Quiesce,
    NullMove,
    ReverseFutilityPruning,
    Checkmate,
    Stalemate,
    RuleDraw,
    Stopped,
}

impl From<Option<ReturnKind>> for ReturnKind {
    fn from(opt: Option<ReturnKind>) -> Self {
        opt.unwrap_or(ReturnKind::Stopped)
    }
}

impl ReturnKindTrait for ReturnKind {}

impl Display for ReturnKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ReturnKind::Pv(mv) => write!(f, "PV: {mv}"),
            ReturnKind::FailHigh(mv) => write!(f, "Fail High: {mv}"),
            ReturnKind::FailLow(mv) => write!(f, "Fail Low: {mv}"),
            ReturnKind::TTExact(mv) => write!(f, "TT Exact: {mv}"),
            ReturnKind::TTUpper(mv) => write!(f, "TT Upper: {mv}"),
            ReturnKind::TTLower(mv) => write!(f, "TT Lower: {mv}"),
            ReturnKind::Quiesce => write!(f, "Quiesce"),
            ReturnKind::ReverseFutilityPruning => write!(f, "RFP"),
            ReturnKind::NullMove => write!(f, "Null"),
            ReturnKind::Checkmate => write!(f, "Checkmate"),
            ReturnKind::Stalemate => write!(f, "Stalemate"),
            ReturnKind::RuleDraw => write!(f, "Rule Draw"),
            ReturnKind::Stopped => write!(f, "Stopped"),
        }
    }
}

impl From<ReturnKind> for EmptyReturnKind {
    fn from(_: ReturnKind) -> Self {
        EmptyReturnKind
    }
}

impl From<Option<EmptyReturnKind>> for EmptyReturnKind {
    fn from(_: Option<EmptyReturnKind>) -> Self {
        EmptyReturnKind
    }
}

impl ReturnKindTrait for EmptyReturnKind {}
