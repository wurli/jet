use std::path::PathBuf;

use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand, ValueEnum};

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser, Debug)]
#[command(name = "jet", about = "A Jupyter Kernel REPL Driver", styles = STYLES)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Spawn a Jupyter kernel and open a REPL on it.
    Connect(ConnectArgs),

    /// Attach a REPL to a kernel that's already running, identified by its
    /// connection file. The kernel keeps running after you exit.
    Attach(AttachArgs),

    /// List sessions in the jet data dir.
    List(ListArgs),
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum StatusFilter {
    Open,
    Closed,
    All,
}

#[derive(Parser, Debug)]
pub struct ListArgs {
    /// Which sessions to show.
    #[arg(long, value_enum, default_value_t = StatusFilter::Open)]
    pub status: StatusFilter,

    /// Show sessions for every working directory. Default: only sessions
    /// whose `working_dir` matches the current dir.
    #[arg(long)]
    pub all_dirs: bool,

    /// Emit each session as a JSON object (one per line) with full metadata.
    #[arg(long)]
    pub json: bool,

    #[command(flatten)]
    pub global: GlobalArgs,
}

#[derive(Parser, Debug)]
pub struct GlobalArgs {
    /// File to write logs to. If unset, logging is disabled.
    /// Log level is controlled with `RUST_LOG` (e.g. `RUST_LOG=jet=trace`).
    #[arg(long, global = true)]
    pub log: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct ConnectArgs {
    /// Path to a Jupyter `kernel.json` kernelspec. Argv and language are
    /// taken from the spec; `{connection_file}` placeholders are
    /// substituted with the path we generate.
    /// Example: jet connect ~/Library/Jupyter/kernels/ark/kernel.json
    #[arg(required = true)]
    pub kernelspec: PathBuf,

    /// Override the location of the kernel connection file. Defaults to
    /// `<session-dir>/connection-file.json` inside the jet data dir.
    #[arg(long)]
    pub connection_file: Option<PathBuf>,

    /// Leave the spawned kernel running after jet exits, so a later `jet
    /// attach <connection-file>` can reuse it.
    #[arg(long)]
    pub persist: bool,

    /// Disable kitty graphics; PNGs are reported as `[image/png NxN bytes]`.
    #[arg(long)]
    pub no_graphics: bool,

    /// A name used to identify the client.
    #[arg(long)]
    pub session_name: Option<String>,

    #[command(flatten)]
    pub global: GlobalArgs,
}

#[derive(Parser, Debug)]
pub struct AttachArgs {
    /// Path to the connection file written by an earlier `jet connect
    /// --persist`. Identifies the kernel and carries its HMAC key.
    #[arg(required = true)]
    pub connection_file: PathBuf,

    /// Disable kitty graphics; PNGs are reported as `[image/png NxN bytes]`.
    #[arg(long)]
    pub no_graphics: bool,

    /// A name used to identify the client.
    #[arg(long)]
    pub session_name: Option<String>,

    #[command(flatten)]
    pub global: GlobalArgs,
}
