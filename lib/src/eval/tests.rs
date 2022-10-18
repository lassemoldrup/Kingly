use crate::eval::material::get_material_score;
use crate::position::Position;

#[test]
fn starting_pos_material_should_be_0() {
    let position = Position::new();

    assert_eq!(get_material_score(&position), 0);
}

#[test]
fn material_score_in_endgame_position() {
    let position = Position::from_fen("6k1/4rp2/6p1/1B6/8/3Q4/1K3PN1/8 w - - 0 1").unwrap();

    assert_eq!(get_material_score(&position), 900);

    let position = Position::from_fen("6k1/4rp2/6p1/1B6/8/3Q4/1K3PN1/8 b - - 0 1").unwrap();

    assert_eq!(get_material_score(&position), -900);
}
