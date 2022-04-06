use std::fs::File;
use std::path::PathBuf;

use crusty::eval::StandardEval;
use serde::Deserialize;
use serde_json::from_reader;

use super::Client;

#[derive(Deserialize)]
struct PerftPosition {
    depth: u32,
    nodes: u64,
    fen: String,
}

#[test]
fn test_perft() {
    let mut client = Client::<StandardEval>::new();
    client.init();

    let mut test_path = PathBuf::new();
    test_path.push(env!("CARGO_MANIFEST_DIR"));
    test_path.push("resources");
    test_path.push("test");
    test_path.push("perft_positions.json");

    let test_file = File::open(test_path).unwrap();

    let tests: Vec<PerftPosition> = from_reader(test_file).unwrap();

    println!("Testing Perft..");
    for (i, test) in tests.iter().enumerate() {
        client.position(&test.fen, &[]).unwrap();
        println!("Running test position {}..", i + 1);
        assert_eq!(client.perft(test.depth), test.nodes);
    }
    println!("All Perft test positions passed")
}
