use super::*;

#[test]
fn test_parse_debug_on() {
    let input = "debug on";
    let expected = Command::Debug(true);
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_debug_off() {
    let input = "debug off";
    let expected = Command::Debug(false);
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_debug_invalid_option() {
    let input = "debug invalid";
    let expected = ParseCommandError::InvalidOption("invalid".into());
    assert_eq!(input.parse::<Command>(), Err(expected));
}

#[test]
fn test_parse_debug_missing_option() {
    let input = "debug";
    let expected = ParseCommandError::MissingOption;
    assert_eq!(input.parse::<Command>(), Err(expected));
}

#[test]
fn test_parse_isready() {
    let input = "isready";
    let expected = Command::IsReady;
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_setoption_hash() {
    let input = "setoption name Hash value 1024";
    let expected = Command::SetOption(UciOption::Hash(1024));
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_setoption_unsupported() {
    let input = "setoption name Unsupported";
    let expected = ParseCommandError::UsupportedOption("Unsupported".into());
    assert_eq!(input.parse::<Command>(), Err(expected));
}

#[test]
fn test_parse_setoption_missing_option() {
    let input = "setoption";
    let expected = ParseCommandError::MissingNameKeyword;
    assert_eq!(input.parse::<Command>(), Err(expected));
}

#[test]
fn test_parse_ucinewgame() {
    let input = "ucinewgame";
    let expected = Command::UciNewGame;
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_position_startpos() {
    let input = "position startpos";
    let expected = Command::Position {
        fen: STARTING_FEN.into(),
        moves: vec![],
    };
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_position_fen() {
    let input = "position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 moves e2e4";
    let expected = Command::Position {
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".into(),
        moves: vec!["e2e4".parse().unwrap()],
    };
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_position_invalid_mode() {
    let input = "position invalid";
    let expected = ParseCommandError::InvalidOption("invalid".into());
    assert_eq!(input.parse::<Command>(), Err(expected));
}

#[test]
fn test_parse_position_missing_option() {
    let input = "position";
    let expected = ParseCommandError::MissingOption;
    assert_eq!(input.parse::<Command>(), Err(expected));
}

#[test]
fn test_parse_go_searchmoves() {
    let input = "go searchmoves e2e4 d2d4";
    let expected = Command::Go(vec![GoOption::SearchMoves(vec![
        "e2e4".parse().unwrap(),
        "d2d4".parse().unwrap(),
    ])]);
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_go_ponder() {
    let input = "go ponder";
    let expected = Command::Go(vec![GoOption::Ponder]);
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_go_wtime() {
    let input = "go wtime 1000";
    let expected = Command::Go(vec![GoOption::WTime(1000)]);
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_go_btime() {
    let input = "go btime 1000";
    let expected = Command::Go(vec![GoOption::BTime(1000)]);
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_go_winc() {
    let input = "go winc 100";
    let expected = Command::Go(vec![GoOption::WInc(100)]);
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_go_binc() {
    let input = "go binc 100";
    let expected = Command::Go(vec![GoOption::BInc(100)]);
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_go_movestogo() {
    let input = "go movestogo 40";
    let expected = Command::Go(vec![GoOption::MovesToGo(40)]);
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_go_depth() {
    let input = "go depth 10";
    let expected = Command::Go(vec![GoOption::Depth(10)]);
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_go_nodes() {
    let input = "go nodes 1000";
    let expected = Command::Go(vec![GoOption::Nodes(1000)]);
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_go_mate() {
    let input = "go mate 5";
    let expected = Command::Go(vec![GoOption::Mate(5)]);
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_go_movetime() {
    let input = "go movetime 1000";
    let expected = Command::Go(vec![GoOption::MoveTime(Duration::from_millis(1000))]);
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_go_infinite() {
    let input = "go infinite";
    let expected = Command::Go(vec![GoOption::Infinite]);
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_go_multiple_options() {
    let input = "go searchmoves e2e4 d2d4 ponder wtime 1000 btime 1000 winc 100 binc 100 movestogo 40 depth 10 nodes 1000 mate 5 movetime 1000";
    let expected = Command::Go(vec![
        GoOption::SearchMoves(vec!["e2e4".parse().unwrap(), "d2d4".parse().unwrap()]),
        GoOption::Ponder,
        GoOption::WTime(1000),
        GoOption::BTime(1000),
        GoOption::WInc(100),
        GoOption::BInc(100),
        GoOption::MovesToGo(40),
        GoOption::Depth(10),
        GoOption::Nodes(1000),
        GoOption::Mate(5),
        GoOption::MoveTime(Duration::from_millis(1000)),
    ]);
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_stop() {
    let input = "stop";
    let expected = Command::Stop;
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_ponderhit() {
    let input = "ponderhit";
    let expected = Command::PonderHit;
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_quit() {
    let input = "quit";
    let expected = Command::Quit;
    assert_eq!(input.parse::<Command>(), Ok(expected));
}

#[test]
fn test_parse_unsupported_command() {
    let input = "unsupported";
    let expected = ParseCommandError::UnsupportedCommand("unsupported".into());
    assert_eq!(input.parse::<Command>(), Err(expected));
}
