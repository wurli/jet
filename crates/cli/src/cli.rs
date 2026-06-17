use std::path::PathBuf;

use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand};

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

    /// Where to write the kernel connection file. Defaults to a tempfile
    /// that is cleaned up on exit. With `--detach`, the kernel outlives
    /// jet and you'll need this path (or a stable one of your choosing)
    /// to reattach later.
    #[arg(long)]
    pub connection_file: Option<PathBuf>,

    /// Leave the spawned kernel running after jet exits, so a later `jet
    /// attach <connection-file>` can reuse it. Requires `--connection-file`.
    #[arg(long, requires = "connection_file")]
    pub detach: bool,

    /// Disable kitty graphics; PNGs are reported as `[image/png NxN bytes]`.
    #[arg(long)]
    pub no_graphics: bool,

    #[command(flatten)]
    pub global: GlobalArgs,
}

#[derive(Parser, Debug)]
pub struct AttachArgs {
    /// Path to the connection file written by an earlier `jet connect
    /// --detach`. Identifies the kernel and carries its HMAC key.
    #[arg(required = true)]
    pub connection_file: PathBuf,

    /// Disable kitty graphics; PNGs are reported as `[image/png NxN bytes]`.
    #[arg(long)]
    pub no_graphics: bool,

    #[command(flatten)]
    pub global: GlobalArgs,
}
