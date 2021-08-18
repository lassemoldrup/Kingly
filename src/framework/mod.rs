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

pub trait Client {
    fn init(&mut self);
    fn is_init(&self) -> bool;
    fn set_position(&mut self, fen: &str) -> Result<(), FenParseError>;
    fn get_moves(&self) -> MoveList;
    fn make_move(&mut self, mv: Move) -> Result<(), String>;
    fn unmake_move(&mut self) -> Result<(), String>;
    fn perft(&self, depth: u32) -> u64;
}

pub trait Searchable<'f> {
    type InfSearch: Search<'f>;
    type DepthSearch: Search<'f>;

    fn search_depth(&self, depth: u32) -> Self::DepthSearch;
    fn search(&self) -> Self::InfSearch;
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
}

pub trait MoveGen<P: Position> {
    fn gen_all_moves(&self, position: &P) -> MoveList;
}

pub trait MoveGenFactory<MG: MoveGen<P>, P: Position> {
    fn create(&self) -> MG;
}

pub trait Eval<P: Position> {
    fn eval(&self, position: &P) -> Value;
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Side {
    KingSide, QueenSide
}