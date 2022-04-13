use std::env;

use cli::Cli;
use client::Client;
use io::Logging;
use itertools::Itertools;
use log::LevelFilter;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};

mod cli;
mod client;
mod io;
mod uci;

fn main() -> std::io::Result<()> {
    if env::args().contains(&String::from("--trace")) {
        TermLogger::init(
            LevelFilter::Trace,
            Config::default(),
            TerminalMode::Stderr,
            ColorChoice::Auto,
        )
        .unwrap();
    }

    let client = Client::new();
    Cli::new(
        client,
        Logging(std::io::stdin()),
        Logging(std::io::stdout()),
    )
    .start()
}
