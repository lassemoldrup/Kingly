use std::fmt::Debug;

use crate::framework::castling::CastlingRights;
use crate::framework::color::Color;
use crate::framework::fen::FenParseError;
use crate::framework::moves::{Move, MoveList};
use crate::framework::piece::Piece;
use crate::framework::search::Search;
use crate::framework::square::Square;
use crate::framework::value::Value;

pub mod square;
pub mod color;
pub mod piece;
pub mod moves;
pub mod castling;
pub mod fen;
pub mod direction;
pub mod square_map;
pub mod square_vec;
pub mod value;
pub mod search;
pub mod util;
pub mod io;
pub mod log;

pub struct NotSupportedError;
pub trait Client {
    type Search<'client, 'f>: Search<'f> where Self: 'client;

    fn init(&mut self);
    fn is_init(&self) -> bool;
    fn set_position(&mut self, fen: &str) -> Result<(), FenParseError>;
    fn get_moves(&self) -> MoveList;
    fn make_move(&mut self, mv: Move) -> Result<(), String>;
    fn unmake_move(&mut self) -> Result<(), String>;
    fn perft(&self, depth: u32) -> u64;
    fn search<'client, 'f>(&'client mut self) -> Self::Search<'client, 'f>;
    fn clear_trans_table(&mut self);
    fn set_hash_size(&mut self, hash_size: usize) -> Result<(), NotSupportedError>;
}

pub trait PieceMap {
    fn get(&self, sq: Square) -> Option<Piece>;
}

pub trait Position {
    type PieceMap: PieceMap;
    fn pieces(&self) -> &Self::PieceMap;
    fn to_move(&self) -> Color;
    fn castling(&self) -> &CastlingRights;
    fn en_passant_sq(&self) -> Option<Square>;
    /// Whether the position is a draw by threefold repetition or fifty-move rule,
    /// i.e. not by stalemate
    fn is_draw(&self) -> bool;
}

pub trait MoveGen<P: Position> {
    fn create() -> Self;
    fn gen_all_moves(&self, position: &P) -> MoveList;
    fn gen_all_moves_and_check(&self, position: &P) -> (MoveList, bool);
    fn gen_captures(&self, position: &P) -> MoveList;
}

pub trait Eval<P: Position> {
    fn create() -> Self;
    fn eval(&self, position: &P) -> Value;
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Side {
    KingSide, QueenSide
}