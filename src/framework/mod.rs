use crate::framework::fen::FenParseError;

pub mod square;
pub mod color;
pub mod piece;
pub mod moves;
pub mod fen;
pub mod direction;
pub mod square_map;
pub mod square_vec;

pub trait Game {
    fn perft(&mut self, depth: u32) -> u64;
    fn set_position(&mut self, fen: &str) -> Result<(), FenParseError>;
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Side {
    KingSide, QueenSide
}