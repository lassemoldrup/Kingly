use std::fmt::{self, Display, Formatter};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use eframe::egui;
use kingly_lib::eval::StandardEval;
use kingly_lib::search::trace::{ReturnKind, SearchObserver};
use kingly_lib::search::{NodeType, SearchJob, ThreadPool};
use kingly_lib::types::{Move, Value};
use kingly_lib::Position;

enum State {
    Input { fen: String, depth: i8 },
    Searching,
    Done,
}

fn main() -> eframe::Result {
    let mut state = State::Input {
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string(),
        depth: 4,
    };
    let mut thread_pool = ThreadPool::new();
    thread_pool
        .set_num_threads(1)
        .expect("search is not running");
    let forest = Arc::new(Mutex::new(Forest::default()));

    let options = eframe::NativeOptions::default();
    eframe::run_simple_native("Kingly Trace", options, move |ctx, _frame| {
        if thread_pool.is_running() {
            state = State::Searching;
        } else {
            if thread_pool.wait().is_some() {
                state = State::Done;
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| match &mut state {
            State::Input { fen, depth } => {
                show_input(ui, fen, depth, &mut thread_pool, forest.clone())
            }
            State::Searching => {
                ui.label("Searching...");
                Ok(())
            }
            State::Done => {
                show_forest(ui, &mut forest.lock().unwrap());
                Ok(())
            }
        });
    })
}

fn show_input(
    ui: &mut egui::Ui,
    fen: &mut String,
    depth: &mut i8,
    thread_pool: &mut ThreadPool<StandardEval, Arc<Mutex<Forest>>>,
    forest: Arc<Mutex<Forest>>,
) -> anyhow::Result<()> {
    ui.heading("Input trace parameters");
    ui.horizontal(|ui| {
        let label = ui.label("FEN: ");
        ui.text_edit_singleline(fen).labelled_by(label.id);
    });
    ui.add(egui::Slider::new(depth, 1..=6).text("depth"));
    if ui.button("Trace").clicked() {
        let fen = fen.trim().to_string();
        trace(&fen, *depth, thread_pool, forest)?;
    }
    Ok(())
}

fn trace(
    fen: &str,
    depth: i8,
    thread_pool: &mut ThreadPool<StandardEval, Arc<Mutex<Forest>>>,
    forest: Arc<Mutex<Forest>>,
) -> anyhow::Result<()> {
    let position = Position::from_fen(fen).context("Failed to parse FEN")?;
    let job = SearchJob::default_builder()
        .position(position)
        .depth(depth)
        .observer(forest.clone())
        .build();
    thread_pool.run(job).context("Failed to run search")?;
    Ok(())
}

fn show_forest(ui: &mut egui::Ui, forest: &mut Forest) {
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let roots = forest.roots.clone();
            for root in roots {
                show_node(ui, forest, root);
            }
        });
}

fn show_node(ui: &mut egui::Ui, forest: &mut Forest, index: usize) {
    let node_data = &forest.nodes[index];
    ui.horizontal(|ui| {
        if !forest.children[index].is_empty() {
            let button_label = if forest.expanded[index] { "-" } else { "+" };
            if ui.button(button_label).clicked() {
                forest.expanded[index] = !forest.expanded[index];
            }
        }
        ui.label(&format!("{node_data}"));
    });
    if forest.expanded[index] {
        ui.indent(index, |ui| {
            let children = forest.children[index].clone();
            for child in children {
                show_node(ui, forest, child);
            }
        });
    }
}

#[derive(Default)]
struct Forest {
    roots: Vec<usize>,
    nodes: Vec<NodeData>,
    children: Vec<Vec<usize>>,
    node_stack: Vec<usize>,
    expanded: Vec<bool>,
}

impl SearchObserver for Forest {
    type ReturnKind = ReturnKind;

    fn on_depth(&mut self, _depth: i8) {
        *self = Self::default();
    }

    fn on_node_enter<N: NodeType>(
        &mut self,
        // ignore, we are only using one thread
        _worker_id: usize,
        alpha: Value,
        beta: Value,
        mv: Option<Move>,
        _pvs_re_search: bool,
    ) {
        let node = self.nodes.len();
        if let Some(&parent) = self.node_stack.last() {
            self.children[parent].push(node);
        } else {
            self.roots.push(node);
        }
        let node_kind = if N::IS_ROOT {
            NodeKind::Root
        } else if N::IS_PV {
            NodeKind::Pv(mv.unwrap())
        } else {
            NodeKind::NonPv(mv.unwrap())
        };
        self.nodes.push(NodeData::PartialNode {
            alpha,
            beta,
            node_kind,
        });
        self.children.push(Vec::new());
        self.expanded.push(false);
        self.node_stack.push(node);
    }

    fn on_node_exit<N: NodeType>(
        &mut self,
        _worker_id: usize,
        _mv: Option<Move>,
        ret: Self::ReturnKind,
        score: Option<Value>,
    ) {
        let node = self.node_stack.pop().unwrap();
        let node_data = &mut self.nodes[node];
        let NodeData::PartialNode {
            alpha,
            beta,
            node_kind,
        } = &node_data
        else {
            panic!("{node} was not partial");
        };
        *node_data = NodeData::Node {
            alpha: *alpha,
            beta: *beta,
            node_kind: *node_kind,
            return_kind: ret,
            score,
        };
    }
}

enum NodeData {
    PartialNode {
        alpha: Value,
        beta: Value,
        node_kind: NodeKind,
    },
    Node {
        alpha: Value,
        beta: Value,
        node_kind: NodeKind,
        return_kind: ReturnKind,
        score: Option<Value>,
    },
}

impl Display for NodeData {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            NodeData::PartialNode { .. } => panic!("Partial node"),
            NodeData::Node {
                alpha,
                beta,
                node_kind: NodeKind::Root,
                return_kind,
                score,
            } => {
                write!(f, "Asp. ({alpha:?}, {beta:?}) -> ")?;
                write_return(f, return_kind, *score)
            }
            NodeData::Node {
                alpha,
                beta,
                node_kind: NodeKind::Pv(mv),
                return_kind,
                score,
            } => {
                write!(f, "{mv} PV ({alpha:?}, {beta:?}) -> ")?;
                write_return(f, return_kind, *score)
            }
            NodeData::Node {
                alpha,
                beta,
                node_kind: NodeKind::NonPv(mv),
                return_kind,
                score,
            } => {
                assert!(*beta == *alpha + Value::centipawn(1));
                write!(f, "{mv} Non-PV ({beta:?}) -> ")?;
                write_return(f, return_kind, *score)
            }
        }
    }
}

fn write_return(f: &mut Formatter, return_kind: &ReturnKind, score: Option<Value>) -> fmt::Result {
    if let Some(score) = score {
        write!(f, "{score:?} {return_kind}")
    } else {
        write!(f, "Stopped")
    }
}

#[derive(Clone, Copy)]
enum NodeKind {
    Root,
    Pv(Move),
    NonPv(Move),
}
