use std::fmt::Debug;

use crate::framework::fen::FenParseError;
use crate::framework::moves::{Move, MoveList};
use crate::framework::color::Color;

pub mod square;
pub mod color;
pub mod piece;
pub mod moves;
pub mod fen;
pub mod direction;
pub mod square_map;
pub mod square_vec;
pub mod value;
pub mod util;

pub trait Game : Debug {
    fn perft(&self, depth: u32) -> u64;
    fn get_moves(&self) -> MoveList;
    fn to_move(&self) -> Color;
    fn make_move(&mut self, mv: Move) -> Result<(), ()>;
    fn unmake_move(&mut self) -> Result<(), ()>;
    fn search(&self, depth: u32) -> Move;
    fn search_moves(&self, depth: u32, moves: Vec<Move>) -> Move;
    fn set_position(&mut self, fen: &str) -> Result<(), FenParseError>;
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Side {
    KingSide, QueenSide
}