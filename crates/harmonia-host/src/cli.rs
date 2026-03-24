use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "harmonia", version, about = "Personal media system")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start the media server
    Serve(ServeArgs),
    /// Database management
    Db(DbArgs),
    /// Play a local audio file
    Play(PlayArgs),
    /// Run as a renderer (discovers and pairs with a harmonia server)
    Render(RenderArgs),
}

#[derive(Args)]
pub struct ServeArgs {
    /// Path to harmonia.toml
    #[arg(short, long, default_value = "harmonia.toml")]
    pub config: PathBuf,

    /// Listen address override
    #[arg(long)]
    pub listen: Option<String>,

    /// Port override
    #[arg(short, long)]
    pub port: Option<u16>,
}

#[derive(Args)]
pub struct DbArgs {
    /// Database subcommand
    #[command(subcommand)]
    pub command: DbCommand,
}

#[derive(Args)]
pub struct PlayArgs {
    /// Path to an audio file
    pub file: PathBuf,

    /// Audio output device name (uses default if omitted)
    #[arg(long)]
    pub device: Option<String>,
}

#[derive(Args)]
pub struct RenderArgs {
    /// Explicit server address (skips mDNS discovery)
    #[arg(long)]
    pub server: Option<SocketAddr>,

    /// Directory for TLS certificates and pairing credentials
    #[arg(long, default_value = "~/.config/harmonia/renderer")]
    pub cert_dir: PathBuf,
}

#[derive(Subcommand)]
pub enum DbCommand {
    /// Run pending migrations
    Migrate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serve_defaults() {
        let cli = Cli::parse_from(["harmonia", "serve"]);
        let Command::Serve(args) = cli.command else {
            panic!("expected Serve command");
        };
        assert_eq!(args.config, PathBuf::from("harmonia.toml"));
        assert!(args.listen.is_none());
        assert!(args.port.is_none());
    }

    #[test]
    fn serve_with_overrides() {
        let cli = Cli::parse_from([
            "harmonia",
            "serve",
            "-c",
            "/etc/harmonia.toml",
            "-p",
            "9000",
            "--listen",
            "127.0.0.1",
        ]);
        let Command::Serve(args) = cli.command else {
            panic!("expected Serve command");
        };
        assert_eq!(args.config, PathBuf::from("/etc/harmonia.toml"));
        assert_eq!(args.listen.as_deref(), Some("127.0.0.1"));
        assert_eq!(args.port, Some(9000));
    }

    #[test]
    fn render_with_server_parses() {
        let cli = Cli::parse_from(["harmonia", "render", "--server", "127.0.0.1:4433"]);
        let Command::Render(args) = cli.command else {
            panic!("expected Render command");
        };
        assert_eq!(
            args.server,
            Some("127.0.0.1:4433".parse().unwrap())
        );
    }

    #[test]
    fn render_without_server_uses_discovery() {
        let cli = Cli::parse_from(["harmonia", "render"]);
        let Command::Render(args) = cli.command else {
            panic!("expected Render command");
        };
        assert!(args.server.is_none());
    }

    #[test]
    fn db_migrate_parses() {
        let cli = Cli::parse_from(["harmonia", "db", "migrate"]);
        let Command::Db(db) = cli.command else {
            panic!("expected Db command");
        };
        assert!(matches!(db.command, DbCommand::Migrate));
    }
}
