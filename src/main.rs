use cli::Cli;
use client::Client;

mod cli;
mod client;
mod io;
mod uci;

fn main() -> std::io::Result<()> {
    let client = Client::new();
    Cli::new(client, std::io::stdin(), std::io::stdout()).start()
}
