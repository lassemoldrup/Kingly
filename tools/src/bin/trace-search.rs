use std::fmt::{self, Display, Formatter};
use std::sync::atomic::AtomicBool;
use std::{env, process};

use eframe::egui::{CentralPanel, Context, ScrollArea, Ui};
use eframe::{App, Frame, NativeOptions};
use kingly_lib::eval::StandardEval;
use kingly_lib::move_gen::MoveGen;
use kingly_lib::position::Position;
use kingly_lib::search::{AspirationResult, Observer, ReturnKind, Search, TranspositionTable};
use kingly_lib::tables::Tables;
use kingly_lib::types::{Move, Value};

#[derive(Clone, Copy, Debug)]
enum NodeData {
    Search {
        mv: Move,
        alpha: Value,
        beta: Value,
        score: Value,
        kind: ReturnKind,
    },
    Aspiration {
        mv: Move,
        alpha: Value,
        beta: Value,
        prev: Value,
    },
    AspIteration {
        low: Value,
        high: Value,
        score: Value,
        kind: ReturnKind,
        result: AspirationResult,
    },
    PartialRoot,
    Partial {
        mv: Move,
        alpha: Value,
        beta: Value,
    },
    PartialAspIteration1 {
        low: Value,
        high: Value,
    },
    PartialAspIteration2 {
        low: Value,
        high: Value,
        score: Value,
        kind: ReturnKind,
    },
    Root {
        mv: Move,
        score: Value,
    },
}

impl Display for NodeData {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            NodeData::Search {
                mv,
                alpha,
                beta,
                score,
                kind,
            } => write!(f, "{mv} [{alpha}; {beta}] -> {score} ({kind})"),
            NodeData::Aspiration {
                mv,
                alpha,
                beta,
                prev,
            } => write!(f, "{mv} [{alpha}; {beta}] (asp. {prev})"),
            NodeData::AspIteration {
                low,
                high,
                score,
                kind,
                result,
            } => write!(f, "{result} [{low}; {high}] -> {score} ({kind})"),
            NodeData::Root { mv, score } => write!(f, "Result: {mv} -> {score}"),
            _ => panic!("Attempt to display partial node"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Node {
    data: NodeData,
    first_child: usize,
    last_child: usize,
    next_sibling: usize,
}

impl Node {
    fn new(data: NodeData) -> Self {
        Self {
            data,
            first_child: 0,
            last_child: 0,
            next_sibling: 0,
        }
    }
}

/// A tree that models the search tree of the engine.
/// The root of the tree holds no data and will point to some amount of `Aspiration` nodes.
/// Each `Aspiration` node will have `AspirationIteration`s as children.
/// Each of these will have a number of `Search` nodes as children.
#[derive(Debug)]
struct Tree {
    tree: Vec<Node>,
    ancestors: Vec<usize>,
}

impl Tree {
    fn new() -> Self {
        let root = Node::new(NodeData::PartialRoot);

        Self {
            tree: vec![root],
            ancestors: vec![0],
        }
    }

    fn push_node(&mut self, data: NodeData) {
        self.tree.push(Node::new(data));
        let index = self.tree.len() - 1;

        let parent = &mut self.tree[*self.ancestors.last().unwrap()];
        let sibling_index = parent.last_child;

        parent.last_child = index;
        if sibling_index == 0 {
            parent.first_child = index;
        } else {
            self.tree[sibling_index].next_sibling = index;
        }
    }

    fn children(&self, index: usize) -> Children {
        Children {
            tree: &self.tree,
            next_child: self.tree[index].first_child,
        }
    }
}

impl Observer for &mut Tree {
    fn new_depth(&mut self, _: u8) {
        **self = Tree::new();
    }

    fn move_made(&mut self, mv: Move, alpha: Value, beta: Value) {
        let data = NodeData::Partial { alpha, beta, mv };
        self.push_node(data);
        self.ancestors.push(self.tree.len() - 1);
    }

    fn move_unmade(&mut self, _: Move) {
        self.ancestors.pop();
    }

    fn score_found(&mut self, score: Value, kind: ReturnKind) {
        let node = &mut self.tree[*self.ancestors.last().unwrap()];
        match node.data {
            NodeData::Partial { mv, alpha, beta } => {
                node.data = NodeData::Search {
                    mv,
                    alpha,
                    beta,
                    score,
                    kind,
                };
            }
            NodeData::PartialAspIteration1 { low, high } => {
                node.data = NodeData::PartialAspIteration2 {
                    low,
                    high,
                    score,
                    kind,
                };
            }
            NodeData::PartialRoot => {
                let mv = match kind {
                    ReturnKind::Best(mv) | ReturnKind::Beta(mv) => mv,
                    _ => panic!("The root move will always be Best or Beta"),
                };
                node.data = NodeData::Root { mv, score };
            }
            _ => panic!(
                "Expected partial, partial asp. or partial root node, found: {:?}",
                node
            ),
        }
    }

    fn aspiration_start(&mut self, prev: Value) {
        let node = self.tree.last_mut().unwrap();

        match node.data {
            NodeData::Partial { mv, alpha, beta } => {
                node.data = NodeData::Aspiration {
                    mv,
                    alpha,
                    beta,
                    prev,
                };
            }
            _ => panic!("Expected partial node, found: {:?}", node),
        }
    }

    fn aspiration_iter_start(&mut self, low: Value, high: Value) {
        let data = NodeData::PartialAspIteration1 { low, high };
        self.push_node(data);

        self.ancestors.push(self.tree.len() - 1);
    }

    fn aspiration_iter_end(&mut self, result: AspirationResult) {
        let node = &mut self.tree[self.ancestors.pop().unwrap()];

        match node.data {
            NodeData::PartialAspIteration2 {
                low,
                high,
                score,
                kind,
            } => {
                node.data = NodeData::AspIteration {
                    low,
                    high,
                    score,
                    result,
                    kind,
                };
            }
            _ => panic!("Expected partial asp. node, found: {:?}", node),
        }
    }
}

struct TreeApp {
    tree: Tree,
    expanded: Vec<bool>,
}

impl TreeApp {
    fn new(tree: Tree) -> Self {
        let len = tree.tree.len();
        Self {
            tree,
            expanded: vec![false; len],
        }
    }

    fn button_label(expanded: &[bool], index: usize) -> &'static str {
        if expanded[index] {
            "-"
        } else {
            "+"
        }
    }

    fn display_node(tree: &Tree, expanded: &mut [bool], ui: &mut Ui, index: usize) {
        for (index, data) in tree.children(index) {
            let label = Self::button_label(expanded, index);

            ui.horizontal(|ui| {
                ui.label(&format!("{}", data));

                if tree.tree[index].first_child != 0 && ui.button(label).clicked() {
                    expanded[index] = !expanded[index];
                }
            });

            if expanded[index] {
                ui.indent(index, |ui| Self::display_node(tree, expanded, ui, index));
            }
        }
    }
}

impl App for TreeApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.label(&format!("{}", self.tree.tree[0].data));
                    Self::display_node(&self.tree, &mut self.expanded, ui, 0);
                })
        });
    }
}

struct Children<'a> {
    tree: &'a [Node],
    next_child: usize,
}

impl<'a> Iterator for Children<'a> {
    type Item = (usize, NodeData);

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_child == 0 {
            None
        } else {
            let node = self.tree[self.next_child];
            let res = (self.next_child, node.data);
            self.next_child = node.next_sibling;
            Some(res)
        }
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();

    if args.len() != 3 {
        println!("Usage: trace-search fen depth");
        process::exit(1);
    }

    let position = Position::from_fen(&args[1]).unwrap();
    let move_gen = MoveGen::new(Tables::get());
    let eval = StandardEval::new(Tables::get());
    let trans_table = TranspositionTable::new();
    let depth: u8 = args[2].parse().unwrap();

    let mut tree = Tree::new();

    Search::new(position, move_gen, eval, &trans_table)
        .register(&mut tree)
        .depth(depth)
        .start(&AtomicBool::new(false));

    let app = TreeApp::new(tree);

    let options = NativeOptions::default();
    eframe::run_native(
        "Kingly: trace search",
        options,
        Box::new(|_cc| Box::new(app)),
    );
}
