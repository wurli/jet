use std::path::PathBuf;

use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand};

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser, Debug)]
#[command(name = "jet", about = "kallichore-backed REPL with kitty graphics", styles = STYLES)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Open a REPL connected to a Jupyter kernel.
    Connect(ConnectArgs),

    /// List active sessions on a kcserver.
    ListSessions(ListSessionsArgs),

    /// Stop a session, or stop all sessions and shut down the kcserver.
    Stop(StopArgs),
}

#[derive(Parser, Debug)]
pub struct ListSessionsArgs {
    /// Emit sessions as a JSON array instead of a table. Includes more detail.
    #[arg(long)]
    pub json: bool,

    #[command(flatten)]
    pub kc: KcArgs,
}

#[derive(Parser, Debug)]
pub struct StopArgs {
    /// Session ID to stop. If omitted, all sessions are killed and the
    /// kcserver itself is shut down.
    #[arg(long)]
    pub session: Option<String>,

    #[command(flatten)]
    pub kc: KcArgs,
}

#[derive(Parser, Debug)]
#[command(next_help_heading = "Kallichore")]
pub struct KcArgs {
    /// Connect to an existing kcserver using this connection file, or spawn
    /// a new one there if none is running.
    #[arg(long)]
    pub kcfile: Option<PathBuf>,

    /// Path to the kcserver binary.
    #[arg(long, env = "JET_KCSERVER", default_value = "kcserver")]
    pub kcserver: String,
}

#[derive(Parser, Debug)]
pub struct ConnectArgs {
    /// Path to a Jupyter `kernel.json` kernelspec. Argv and language are
    /// taken from the spec; `{connection_file}` placeholders are forwarded
    /// to kallichore.
    /// Example: jet connect ~/Library/Jupyter/kernels/ark/kernel.json
    #[arg(required = true)]
    pub kernelspec: PathBuf,

    /// Leave any kcserver this process spawned running after jet exits, so
    /// later invocations can reconnect to the same kernel. Requires
    /// `--kcfile`, since reconnecting needs a known connection file.
    #[arg(long, requires = "kcfile")]
    pub persist: bool,

    /// Disable kitty graphics; PNGs are reported as `[image/png NxN bytes]`.
    #[arg(long)]
    pub no_graphics: bool,

    /// File to write logs to. If unset, logging is disabled.
    /// Log level is controlled with `RUST_LOG` (e.g. `RUST_LOG=jet=trace`).
    #[arg(long)]
    pub log: Option<PathBuf>,

    #[command(flatten)]
    pub kc: KcArgs,
}
