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
    #[command(alias = "c")]
    Connect(ConnectArgs),

    /// Attach a REPL to a kernel that's already running, identified by its
    /// connection file. The kernel keeps running after you exit.
    #[command(alias = "a")]
    Attach(AttachArgs),

    /// List Jupyter sessions tracked by jet.
    #[command(alias = "ls")]
    ListSessions(ListArgs),

    /// List Jupyter kernels discoverable on disk.
    #[command(alias = "lk")]
    ListKernels(ListKernelsArgs),

    /// Stop a running kernel
    #[command(alias = "s")]
    Stop(StopArgs),
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
pub struct ListKernelsArgs {
    /// Emit kernelspec paths as a JSON array.
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
    /// substituted with the path we generate. If omitted, an interactive
    /// picker is shown over the kernels discovered on disk (same set as
    /// `jet list-kernels`).
    /// Example: jet connect ~/Library/Jupyter/kernels/ark/kernel.json
    pub kernelspec: Option<PathBuf>,

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
#[command(group(clap::ArgGroup::new("target").args(["session_id", "connection_file"])))]
pub struct AttachArgs {
    /// Session id (directory name under the jet data dir) to attach to.
    /// Look these up with `jet list`. Mutually exclusive with
    /// `--connection-file`. If neither is given, an interactive picker
    /// is shown over open sessions in the current working directory.
    pub session_id: Option<String>,

    /// Path to a connection file, e.g. written by an earlier `jet connect --persist`. Use this to attach to a kernel that wasn't tracked as a jet session. Mutually exclusive with the positional `session_id`.
    // If the path resolves to a tracked session (via
    // `SessionStore::find_by_connection_file`), behavior matches passing
    // `session_id` directly. For untracked paths there's no id to flip,
    // so a kernel death during the REPL leaves session.json (if any)
    // Open until the next `jet list` runs its `probe_open` self-heal.
    #[arg(long)]
    pub connection_file: Option<PathBuf>,

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
#[command(group(clap::ArgGroup::new("target").args(["session_id", "connection_file"])))]
pub struct StopArgs {
    /// Session id (directory name under the jet data dir) to stop.
    /// Look these up with `jet list`. Mutually exclusive with
    /// `--connection-file`. If neither is given, an interactive picker
    /// is shown over open sessions in the current working directory.
    pub session_id: Option<String>,

    /// Path to a connection file, e.g. written by an earlier `jet connect
    /// --persist`. Use this to stop a kernel that wasn't tracked
    /// as a jet session. Mutually exclusive with the positional
    /// `session_id`.
    #[arg(long)]
    pub connection_file: Option<PathBuf>,

    /// A name used to identify the client shutting down the kernel.
    #[arg(long)]
    pub session_name: Option<String>,

    #[command(flatten)]
    pub global: GlobalArgs,
}
