mod cli;
mod db;
mod error;
mod mcp;
mod migrate;
mod paths;
mod play;
pub mod render;
mod serve;
mod shutdown;
mod startup;

use clap::Parser;
use cli::{Cli, Command};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    // WHY: #529 — a pre-locked `StdoutLock` held across the whole `Serve`
    // arm (which runs for the server's entire lifetime) deadlocked every
    // OTHER thread's `tracing` output the instant a concurrently-scheduled
    // task (verified: the SIGHUP reload handler) tried to log — `main`'s own
    // future never migrates threads (`block_on` drives it on the calling
    // thread only), so its held guard just sat there forever, invisible
    // until a genuinely-concurrent logger contended for the same reentrant
    // lock. An unlocked `Stdout` handle locks/releases per call instead, so
    // no thread holds it across an await point.
    let mut stdout = std::io::stdout();

    let result = match cli.command {
        Command::Serve(args) => serve::run_serve(args, &mut stdout).await,
        Command::Db(db_args) => match db_args.command {
            cli::DbCommand::Migrate(args) => db::run_db_migrate(args, &mut stdout).await,
        },
        // WHY the fresh tokens: the CLI has no cancellation source — Ctrl+C
        // terminates the process directly and the renderer's own signal
        // handlers cover graceful stops — so the interactive subcommands
        // pass a token that never fires. Only the MCP surface (mcp.rs)
        // passes a live one (`RequestContext.ct`).
        Command::Play(args) => play::run_play(args, &mut stdout, CancellationToken::new()).await,
        Command::Render(args) => {
            render::run_render(
                render::RenderArgs {
                    server: args.server,
                    cert_dir: args
                        .cert_dir
                        .unwrap_or_else(paths::default_renderer_cert_dir),
                    name: args.name,
                    config_path: args.config,
                },
                CancellationToken::new(),
            )
            .await
        }
        Command::Migrate(args) => migrate::run_migrate(args, &mut stdout).await,
        Command::Mcp(args) => mcp::run_stdio(args.config).await,
    };

    if let Err(e) = result {
        eprintln!("fatal: {e}");
        std::process::exit(1);
    }
}
