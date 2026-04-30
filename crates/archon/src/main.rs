mod cli;
mod error;
mod migrate;
mod play;
pub mod render;
mod serve;
mod shutdown;
mod startup;

use clap::Parser;
use cli::{Cli, Command};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();

    let result = match cli.command {
        Command::Serve(args) => serve::run_serve(args, &mut stdout_lock).await,
        Command::Db(db_args) => match db_args.command {
            cli::DbCommand::Migrate => {
                eprintln!("Database migration runs automatically on serve startup.");
                Ok(())
            }
        },
        Command::Play(args) => play::run_play(args, &mut stdout_lock).await,
        Command::Render(args) => {
            render::run_render(render::RenderArgs {
                server: args.server,
                cert_dir: args.cert_dir,
                name: args.name,
                config_path: args.config,
            })
            .await
        }
        Command::Migrate(args) => migrate::run_migrate(args, &mut stdout_lock).await,
    };

    if let Err(e) = result {
        eprintln!("fatal: {e}");
        std::process::exit(1);
    }
}
