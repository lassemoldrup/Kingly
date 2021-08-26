use std::convert::TryFrom;
use std::iter::repeat;

use take_until::TakeUntilExt;
use bitintr::Pdep;

use crate::bb;
use crate::framework::square_map::SquareMap;
use crate::standard::Bitboard;
use crate::framework::square_vec::SquareVec;
use crate::framework::square::Square;

#[cfg(test)]
mod tests;

lazy_static! {
    static ref TABLES: Tables = Tables::new();
}

pub struct Tables {
    pub white_pawn_attacks: SquareMap<Bitboard>,
    pub black_pawn_attacks: SquareMap<Bitboard>,
    pub knight_attacks: SquareMap<Bitboard>,
    pub king_attacks: SquareMap<Bitboard>,
    pub bishop_masks: SquareMap<Bitboard>, // Relevant occupancy squares for bishop attacks
    pub rook_masks: SquareMap<Bitboard>, // Same for rooks
    pub slider_attacks: Vec<Bitboard>,
    pub bishop_offsets: SquareMap<usize>,
    pub rook_offsets: SquareMap<usize>,
    pub line_through: SquareMap<SquareMap<Bitboard>>, // The entire line between two squares, empty bb if not on line
    pub ray_to: SquareMap<SquareMap<Bitboard>>, // All the squares between two squares, includes the
    // destination square, empty bb if not on a line
}

impl Tables {
    pub fn get() -> &'static Self {
        &TABLES
    }

    fn new() -> Self {
        let white_pawn_attacks = Self::init_white_pawn_attacks();
        let black_pawn_attacks = Self::init_black_pawn_attacks();
        let knight_attacks = Self::init_knight_attacks();
        let king_attacks = Self::init_king_attacks();
        let bishop_masks = Self::init_bishop_masks();
        let rook_masks = Self::init_rook_masks();
        let line_through = Self::init_line_through();
        let ray_to = Self::init_ray_to();

        let mut tables = Self {
            white_pawn_attacks,
            black_pawn_attacks,
            knight_attacks,
            king_attacks,
            bishop_masks,
            rook_masks,
            slider_attacks: Vec::with_capacity(107_648),
            bishop_offsets: SquareMap::new([0; 64]),
            rook_offsets: SquareMap::new([0; 64]),
            line_through,
            ray_to,
        };

        tables.init_slider_attacks();
        tables
    }

    fn init_white_pawn_attacks() -> SquareMap<Bitboard> {
        let move_vecs = vec![SquareVec(1, 1), SquareVec(1, -1)];

        Self::init_step_attacks(move_vecs)
    }

    fn init_black_pawn_attacks() -> SquareMap<Bitboard> {
        let move_vecs = vec![SquareVec(-1, 1), SquareVec(-1, -1)];

        Self::init_step_attacks(move_vecs)
    }

    fn init_knight_attacks() -> SquareMap<Bitboard> {
        let move_vecs = vec![
            SquareVec(2, 1), SquareVec(1, 2), SquareVec(-1, 2), SquareVec(-2, 1),
            SquareVec(-2, -1), SquareVec(-1, -2), SquareVec(1, -2), SquareVec(2, -1),
        ];

        Self::init_step_attacks(move_vecs)
    }

    fn init_king_attacks() -> SquareMap<Bitboard> {
        let move_vecs = vec![
            SquareVec(1, 0), SquareVec(1, 1), SquareVec(0, 1), SquareVec(-1, 1),
            SquareVec(-1, 0), SquareVec(-1, -1), SquareVec(0, -1), SquareVec(1, -1),
        ];

        Self::init_step_attacks(move_vecs)
    }

    fn init_step_attacks(move_vecs: Vec<SquareVec>) -> SquareMap<Bitboard> {
        let mut table = SquareMap::new([Bitboard::new(); 64]);

        for (sq, bb) in table.iter_mut() {
            *bb = move_vecs.iter().fold(Bitboard::new(), |acc, vec| {
                match sq + *vec {
                    Some(dest) => acc.add_sq(dest),
                    None => acc,
                }
            });
        }

        table
    }

    const TO_SQ: fn((u8, u8)) -> Square = |(r, f)| Square::try_from(8 * r + f).unwrap();

    fn gen_bishop_attacks_slow(sq: Square, occ: Bitboard) -> Bitboard {
        let mut atk_bb = Bitboard::new();

        let rank = sq.rank();
        let file = sq.file();

        let sq_occ = |sq: &Square| occ.contains(*sq);

        let north_east = (rank+1..8).zip(file+1..8)
            .map(Self::TO_SQ)
            .take_until(sq_occ);
        let south_east = (0..rank).rev().zip(file+1..8)
            .map(Self::TO_SQ)
            .take_until(sq_occ);
        let south_west = (0..rank).rev().zip((0..file).rev())
            .map(Self::TO_SQ)
            .take_until(sq_occ);
        let north_west = (rank+1..8).zip((0..file).rev())
            .map(Self::TO_SQ)
            .take_until(sq_occ);

        for sq in north_east.chain(south_east).chain(south_west).chain(north_west) {
            atk_bb = atk_bb.add_sq(sq);
        }

        atk_bb
    }

    fn gen_rook_attacks_slow(sq: Square, occ: Bitboard) -> Bitboard {
        let mut atk_bb = Bitboard::new();

        let rank = sq.rank();
        let file = sq.file();

        let sq_occ = |sq: &Square| occ.contains(*sq);

        let north = (rank+1..8).zip(repeat(file))
            .map(Self::TO_SQ)
            .take_until(sq_occ);
        let east = repeat(rank).zip(file+1..8)
            .map(Self::TO_SQ)
            .take_until(sq_occ);
        let south = ((0..rank).rev()).zip(repeat(file))
            .map(Self::TO_SQ)
            .take_until(sq_occ);
        let west = repeat(rank).zip((0..file).rev())
            .map(Self::TO_SQ)
            .take_until(sq_occ);

        for sq in north.chain(east).chain(south).chain(west) {
            atk_bb = atk_bb.add_sq(sq);
        }

        atk_bb
    }

    fn init_bishop_masks() -> SquareMap<Bitboard> {
        let mut table = SquareMap::new([Bitboard::new(); 64]);

        for (sq, bb) in table.iter_mut() {
            *bb = Self::gen_bishop_attacks_slow(sq, Bitboard::new())
                - Bitboard::RANKS[0]
                - Bitboard::RANKS[7]
                - Bitboard::FILES[0]
                - Bitboard::FILES[7];
        }

        table
    }

    fn init_rook_masks() -> SquareMap<Bitboard> {
        let mut table = SquareMap::new([Bitboard::new(); 64]);

        for (sq, bb) in table.iter_mut() {
            *bb = Self::gen_rook_attacks_slow(sq, Bitboard::new());
            if sq.rank() != 0 {
                *bb -= Bitboard::RANKS[0];
            }
            if sq.rank() != 7 {
                *bb -= Bitboard::RANKS[7];
            }
            if sq.file() != 0 {
                *bb -= Bitboard::FILES[0];
            }
            if sq.file() != 7 {
                *bb -= Bitboard::FILES[7];
            }
        }

        table
    }

    fn init_slider_attacks(&mut self) {
        let mut table_idx = 0;

        for sq in Square::iter() {
            let count = 1 << self.bishop_masks[sq].len();

            for occ in 0..count as u64 {
                let occ_bb = occ.pdep(self.bishop_masks[sq].into()).into();
                let atk_bb = Self::gen_bishop_attacks_slow(sq, occ_bb);
                self.slider_attacks.push(atk_bb);
            }

            self.bishop_offsets[sq] = table_idx;
            table_idx += count;

            let count = 1 << self.rook_masks[sq].len();

            for occ in 0..count as u64 {
                let occ_bb = occ.pdep(self.rook_masks[sq].into()).into();
                let atk_bb = Self::gen_rook_attacks_slow(sq, occ_bb);
                self.slider_attacks.push(atk_bb);
            }

            self.rook_offsets[sq] = table_idx;
            table_idx += count;
        }
    }

    fn init_line_through() -> SquareMap<SquareMap<Bitboard>> {
        let mut table = SquareMap::new([SquareMap::new([Bitboard::new(); 64]); 64]);

        for from in Square::iter() {
            let from_rank = from.rank() as i8;
            let from_file = from.file() as i8;

            for to in Square::iter() {
                if from == to {
                    continue;
                }

                let to_rank = to.rank() as i8;
                let to_file = to.file() as i8;
                let delta_rank = to_rank - from_rank;
                let delta_file = to_file - from_file;

                if from_rank == to_rank {
                    table[from][to] = Bitboard::RANKS[from.rank() as usize];
                } else if from_file == to_file {
                    table[from][to] = Bitboard::FILES[from.file() as usize];
                } else if delta_rank == delta_file {
                    let diag_idx = (7 + from_rank - from_file) as usize;
                    table[from][to] = Bitboard::DIAGS[diag_idx];
                } else if delta_rank == -delta_file {
                    let anit_diag_idx = (from_rank + from_file) as usize;
                    table[from][to] = Bitboard::ANTI_DIAGS[anit_diag_idx];
                }
            }
        }

        table
    }

    fn init_ray_to() -> SquareMap<SquareMap<Bitboard>> {
        let mut table = SquareMap::new([SquareMap::new([Bitboard::new(); 64]); 64]);

        for from in Square::iter() {
            let from_rank = from.rank() as i8;
            let from_file = from.file() as i8;

            for to in Square::iter() {
                if from == to {
                    table[from][to] = bb!(to);
                    continue;
                }

                let to_rank = to.rank() as i8;
                let to_file = to.file() as i8;
                let delta_rank = to_rank - from_rank;
                let delta_file = to_file - from_file;

                if delta_rank.abs() != delta_file.abs() && delta_rank != 0 && delta_file != 0 {
                    continue;
                }

                let rank_step = delta_rank.signum();
                let file_step = delta_file.signum();

                let mut rank = from_rank + rank_step;
                let mut file = from_file + file_step;
                while rank != to_rank || file != to_file {
                    let sq = Square::try_from((8 * rank + file) as u8).unwrap();
                    table[from][to] = table[from][to].add_sq(sq);

                    rank += rank_step;
                    file += file_step;
                }
                table[from][to] = table[from][to].add_sq(to);
            }
        }

        table
    }
}

