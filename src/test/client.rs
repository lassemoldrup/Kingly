use crate::framework::{Client, Searchable};
use crate::framework::fen::{FenParseError, STARTING_FEN};
use crate::framework::moves::{Move, MoveList};
use crate::framework::search::SearchResult;
use crate::framework::square::Square;
use crate::test::search::SearchStub;

pub struct ClientStub {
    is_init: bool,
    pub last_fen: String,
    pub moves_made: Vec<Move>,
    search_result: SearchResult,
}

impl ClientStub {
    pub fn new(search_result: SearchResult) -> Self {
        Self {
            is_init: false,
            last_fen: STARTING_FEN.to_string(),
            moves_made: vec![],
            search_result
        }
    }
}

impl Client for ClientStub {
    fn init(&mut self) {
        self.is_init = true;
    }

    fn is_init(&self) -> bool {
        self.is_init
    }

    fn set_position(&mut self, fen: &str) -> Result<(), FenParseError> {
        self.last_fen = fen.to_string();
        self.moves_made.clear();
        Ok(())
    }

    fn get_moves(&self) -> MoveList {
        let mut moves = MoveList::new();
        moves.push(Move::Regular(Square::A1, Square::A2));
        moves
    }

    fn make_move(&mut self, mv: Move) -> Result<(), String> {
        self.moves_made.push(mv);
        Ok(())
    }

    fn unmake_move(&mut self) -> Result<(), String> {
        self.moves_made.pop()
            .ok_or_else(|| "No moves to unmake".to_string())
            .map(|_| ())
    }

    fn perft(&self, _depth: u32) -> u64 {
        todo!()
    }
}

impl<'f> Searchable<'f> for &'f ClientStub {
    type Search = SearchStub<'f>;

    fn search(&self) -> Self::Search {
        SearchStub::new(self.search_result.clone())
    }
}