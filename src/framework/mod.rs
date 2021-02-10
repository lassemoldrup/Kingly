use std::fmt::Debug;

use crate::framework::fen::FenParseError;
use crate::framework::moves::{Move, MoveList};

pub mod square;
pub mod color;
pub mod piece;
pub mod moves;
pub mod fen;
pub mod direction;
pub mod square_map;
pub mod square_vec;

pub trait Game : Debug {
    fn perft(&mut self, depth: u32) -> u64;
    fn get_moves(&mut self) -> MoveList;
    fn make_move(&mut self, mv: Move) -> Result<(), ()>;
    fn unmake_move(&mut self) -> Result<(), ()>;
    fn set_position(&mut self, fen: &str) -> Result<(), FenParseError>;
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Side {
    KingSide, QueenSide
}