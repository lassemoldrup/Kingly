#![feature(let_chains)]

#[macro_use]
extern crate lazy_static;

pub mod eval;
pub mod fen;
pub mod move_gen;
pub mod move_list;
pub mod position;
pub mod search;
pub mod square_map;
pub mod tables;
pub mod types;
pub mod util;
pub mod zobrist;
