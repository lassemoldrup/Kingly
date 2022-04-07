use std::fs::File;

use cli::Cli;
use client::Client;
use io::Logging;
use log::LevelFilter;
use simplelog::{Config, WriteLogger};

mod cli;
mod client;
mod io;
mod uci;

fn main() -> std::io::Result<()> {
    WriteLogger::init(
        LevelFilter::Debug,
        Config::default(),
        File::create("./log.txt").unwrap(),
    )
    .unwrap();

    let client = Client::new();
    Cli::new(
        client,
        Logging(std::io::stdin()),
        Logging(std::io::stdout()),
    )
    .start()
}
