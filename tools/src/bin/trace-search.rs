//#![feature(is_some_with)]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::{env, process};

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
    Root(Value),
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
struct Tree {
    tree: Vec<Node>,
    curr_depth: u8,
    ancestors: Vec<usize>,
}

impl Tree {
    fn new() -> Self {
        let root = Node::new(NodeData::Root(Value::centi_pawn(0)));

        Self {
            tree: vec![root],
            curr_depth: 0,
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
}

impl Observer for Tree {
    fn new_depth(&mut self, _: u8) {
        *self = Self::new();
    }

    fn move_made(&mut self, mv: Move, alpha: Value, beta: Value) {
        let data = NodeData::Partial { alpha, beta, mv };
        self.push_node(data);

        self.curr_depth += 1;
        self.ancestors.push(self.tree.len() - 1);
    }

    fn move_unmade(&mut self, _: Move) {
        self.curr_depth -= 1;
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
                assert_eq!(self.curr_depth, 1);
                node.data = NodeData::PartialAspIteration2 {
                    low,
                    high,
                    score,
                    kind,
                };
            }
            NodeData::Root(_) => {
                assert_eq!(self.curr_depth, 0);
                node.data = NodeData::Root(score);
            }
            _ => panic!(
                "Expected partial, partial asp. or root node, found: {:?}",
                node
            ),
        }
    }

    fn aspiration_start(&mut self, prev: Value) {
        assert_eq!(self.curr_depth, 1);

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
        assert_eq!(self.curr_depth, 1);

        let data = NodeData::PartialAspIteration1 { low, high };
        self.push_node(data);

        self.ancestors.push(self.tree.len() - 1);
    }

    fn aspiration_iter_end(&mut self, result: AspirationResult) {
        assert_eq!(self.curr_depth, 1);

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
            _ => panic!("Expected partial asp node, found: {:?}", node),
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
    let mut trans_table = TranspositionTable::new();
    let depth: u8 = args[2].parse().unwrap();

    let tree = Rc::new(RefCell::new(Tree::new()));

    Search::new(position, move_gen, eval, &mut trans_table)
        .depth(depth)
        .register(Rc::downgrade(&tree))
        .start(&AtomicBool::new(false));

    let first = (*tree).borrow().tree[3];
    println!("{:?}", first);

    let mut next = first.first_child;
    while next != 0 {
        let node = (*tree).borrow().tree[next];
        next = node.next_sibling;

        println!("{:?}", node.data);
    }
}
