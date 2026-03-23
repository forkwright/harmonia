mod cli;
mod error;
mod play;
mod serve;
mod shutdown;
mod startup;

use clap::Parser;
use cli::{Cli, Command};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Serve(args) => serve::run_serve(args).await,
        Command::Db(db_args) => match db_args.command {
            cli::DbCommand::Migrate => {
                eprintln!("Database migration runs automatically on serve startup.");
                Ok(())
            }
        },
        Command::Play(args) => play::run_play(args).await,
    };

    if let Err(e) = result {
        eprintln!("fatal: {e}");
        std::process::exit(1);
    }
}
