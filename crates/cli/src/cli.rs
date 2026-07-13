use std::path::PathBuf;

use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand, ValueEnum, crate_version};

pub const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

/// Exit with a clap-styled error attributed to `subcommand` (matching the
/// format clap emits for built-in arg conflicts: usage line, hint, exit
/// code 2). Use this for argument-shape checks that can't be expressed via
/// `ArgGroup` — e.g. when positionals are ambiguous until resolved.
pub fn conflict_exit(subcommand: &str, msg: impl Into<String>) -> ! {
    use clap::CommandFactory;
    use clap::error::ErrorKind;
    let mut cmd = Args::command();
    let sub = cmd
        .find_subcommand_mut(subcommand)
        .unwrap_or_else(|| panic!("subcommand `{subcommand}` exists"));
    sub.error(ErrorKind::ArgumentConflict, msg.into()).exit();
}

#[derive(Parser, Debug)]
#[command(
    name = "jet",
    about = "A Jupyter Kernel REPL Driver",
    styles = STYLES,
    version = crate_version!(),
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Spawn a Jupyter kernel and open a REPL on it.
    #[command(alias = "s")]
    Start(StartArgs),

    /// Attach a REPL to a kernel that's already running, identified by its
    /// connection file. The kernel keeps running after you exit.
    #[command(alias = "a")]
    Attach(AttachArgs),

    /// List Jupyter sessions tracked by jet.
    #[command(alias = "ls")]
    ListSessions(ListSessionsArgs),

    /// List Jupyter kernels discoverable on disk.
    #[command(alias = "lk")]
    ListKernels(ListKernelsArgs),

    /// Stop a running kernel
    #[command()]
    Stop(StopArgs),

    /// Show a session's metadata alongside its kernelspec.
    #[command(alias = "sh")]
    Show(ShowArgs),

    /// Execute code against a running kernel and stream the result to stdout.
    /// Exits once the kernel goes idle for the request.
    #[command(alias = "e")]
    Execute(ExecuteArgs),

    /// Send code to a running kernel and exit immediately. Output (if any)
    /// is discarded — the kernel runs the cell after `jet` has gone. Same
    /// target shape as `jet execute`, minus rendering options.
    #[command(alias = "se")]
    Send(SendArgs),

    /// Print the bundled agent skill documentation (SKILL.md) to stdout.
    #[command()]
    Skill,
}

impl Command {
    pub fn global(&self) -> Option<&GlobalArgs> {
        match self {
            Command::Start(c) => Some(&c.global),
            Command::Attach(c) => Some(&c.global),
            Command::ListSessions(c) => Some(&c.global),
            Command::ListKernels(c) => Some(&c.global),
            Command::Stop(c) => Some(&c.global),
            Command::Execute(c) => Some(&c.global),
            Command::Send(c) => Some(&c.global),
            Command::Show(c) => Some(&c.global),
            Command::Skill => None,
        }
    }
}

/// How to render output from *other* clients sharing this kernel.
///
/// - `Wrap` (default): draw a `┌─name` header and prefix every foreign
///   line with a colored `│ ` gutter, visually boxing the block.
/// - `Prompt`: omit the box entirely. Foreign `execute_input` renders
///   as `name> code` (name colored), and foreign output prints raw
///   with no prefix.
#[derive(Copy, Clone, Debug, ValueEnum, Default, PartialEq, Eq)]
pub enum ExternalClientStyle {
    #[default]
    Wrap,
    Prompt,
}

impl From<ExternalClientStyle> for crate::render::ExternalClientStyle {
    fn from(s: ExternalClientStyle) -> Self {
        match s {
            ExternalClientStyle::Wrap => Self::Wrap,
            ExternalClientStyle::Prompt => Self::Prompt,
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum StatusFilter {
    Open,
    Closed,
    All,
}

// Convert the enum from clap to the internal jet_core enum. This avoids a clap dependency in
// jet_core. In theory that would be fine but feels cleaner this way.
impl From<StatusFilter> for jet_core::manager::StatusFilter {
    fn from(s: StatusFilter) -> Self {
        match s {
            StatusFilter::Open => Self::Open,
            StatusFilter::Closed => Self::Closed,
            StatusFilter::All => Self::All,
        }
    }
}

#[derive(Parser, Debug)]
pub struct ListSessionsArgs {
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
#[command(group(clap::ArgGroup::new("start-target").args(["session_id", "connection_file"])))]
pub struct StartArgs {
    /// Path to a Jupyter `kernel.json` kernelspec. Argv and language are taken from the spec;
    /// `{connection_file}` placeholders are substituted with the path we generate. If
    /// omitted, an interactive picker is shown over the kernels discovered on disk (same set
    /// as `jet list-kernels`). Example: jet start ~/Library/Jupyter/kernels/ark/kernel.json
    pub kernelspec: Option<PathBuf>,

    /// Override the location of the kernel connection file. Defaults to
    /// `<session-dir>/connection-file.json` inside the jet data dir. Passing this flag opts
    /// out of session tracking — no session.json is written and `jet list` won't see the
    /// kernel. Use when another process owns the connection file lifecycle.
    // NOTE: in future we could support an `--adopt` flag which would symlink the connection
    // file into the session dir.
    #[arg(long)]
    pub connection_file: Option<PathBuf>,

    /// Pre-formed session id to use instead of having jet generate one. The id becomes the
    /// session-dir name under the jet data dir, so it must not collide with an existing
    /// session. Use `jet.make_session_id()` from Lua to mint one. Mutually exclusive with
    /// `--connection-file`.
    #[arg(long)]
    pub session_id: Option<String>,

    /// Leave the spawned kernel running after jet exits, so a later `jet attach
    /// <connection-file>` can reuse it.
    #[arg(long)]
    pub persist: bool,

    /// Disable kitty graphics; PNGs are reported as `[image/png NxN bytes]`.
    #[arg(long)]
    pub no_graphics: bool,

    /// Disable automatic indentation in the REPL. This can be useful when pasting code which is
    /// already indented.
    #[arg(long)]
    pub no_indent: bool,

    /// A name used to identify the client.
    #[arg(long, env = "JET_SESSION_NAME")]
    pub session_name: Option<String>,

    /// How to render output from other clients sharing this kernel.
    /// `wrap` (default) draws a boxed block with a `│` gutter;
    /// `prompt` prints `name>` before the input line and leaves the
    /// output unprefixed.
    #[arg(long, value_enum, default_value_t = ExternalClientStyle::default())]
    pub external_client_style: ExternalClientStyle,

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

    /// Path to a connection file, e.g. written by an earlier `jet start --persist`. Use
    /// this to attach to a kernel that wasn't tracked as a jet session. Mutually exclusive
    /// with the positional `session_id`.
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

    /// Render the kernel banner on attach. By default attach suppresses it
    /// so reconnects don't reprint what the original spawn already drew.
    #[arg(long)]
    pub banner: bool,

    /// Disable automatic indentation in the REPL. This can be useful when pasting code which is
    /// already indented.
    #[arg(long)]
    pub no_indent: bool,

    /// A name used to identify the client.
    #[arg(long, env = "JET_SESSION_NAME")]
    pub session_name: Option<String>,

    /// How to render output from other clients sharing this kernel.
    /// `wrap` (default) draws a boxed block with a `│` gutter;
    /// `prompt` prints `name>` before the input line and leaves the
    /// output unprefixed.
    #[arg(long, value_enum, default_value_t = ExternalClientStyle::default())]
    pub external_client_style: ExternalClientStyle,

    #[command(flatten)]
    pub global: GlobalArgs,
}

#[derive(Parser, Debug)]
pub struct ExecuteArgs {
    /// Session id (directory name under the jet data dir) to execute against.
    /// Look these up with `jet list`. Required unless `--connection-file` or
    /// `--kernelspec` is given.
    pub session_id: Option<String>,

    /// Path to a connection file, e.g. written by an earlier `jet start
    /// --persist`. Alternative to the positional `session_id`. When combined
    /// with `--kernelspec`, the file must not already exist — jet writes it
    /// for the kernel it spawns.
    #[arg(long)]
    pub connection_file: Option<PathBuf>,

    /// Path to a Jupyter `kernel.json` kernelspec. When set, jet spawns a
    /// fresh kernel for this execute and shuts it down on exit. Mutually
    /// exclusive with the positional `session_id`.
    #[arg(long)]
    pub kernelspec: Option<PathBuf>,

    /// Code to execute. If omitted, read from stdin.
    pub code: Option<String>,

    /// Set the `silent` flag on the underlying Jupyter `execute_request`.
    /// Kernels typically suppress output (including `display_data`) and
    /// skip history when this is set.
    #[arg(long)]
    pub silent: bool,

    /// Disable kitty graphics; PNGs are reported as `[image/png NxN bytes]`.
    #[arg(long)]
    pub no_graphics: bool,

    /// A name used to identify the client.
    #[arg(long, env = "JET_SESSION_NAME")]
    pub session_name: Option<String>,

    #[command(flatten)]
    pub global: GlobalArgs,
}

#[derive(Parser, Debug)]
pub struct SendArgs {
    /// Session id (directory name under the jet data dir) to send to.
    /// Look these up with `jet list`.
    pub session_id: Option<String>,

    /// Path to a connection file. Alternative to the positional `session_id`.
    /// When combined with `--kernelspec`, the file must not already exist —
    /// jet writes it for the kernel it spawns.
    #[arg(long)]
    pub connection_file: Option<PathBuf>,

    /// Path to a Jupyter `kernel.json` kernelspec. When set, jet spawns a
    /// fresh kernel for this send and shuts it down on exit. Mutually
    /// exclusive with the positional `session_id`.
    #[arg(long)]
    pub kernelspec: Option<PathBuf>,

    /// Code to send. If omitted, read from stdin.
    pub code: Option<String>,

    /// Set the `silent` flag on the underlying Jupyter `execute_request`.
    #[arg(long)]
    pub silent: bool,

    /// A name used to identify the client.
    #[arg(long, env = "JET_SESSION_NAME")]
    pub session_name: Option<String>,

    #[command(flatten)]
    pub global: GlobalArgs,
}

#[derive(Parser, Debug)]
pub struct ShowArgs {
    /// Session id (directory name under the jet data dir) to show.
    /// Look these up with `jet list-sessions`.
    pub session_id: String,

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

    /// Path to a connection file, e.g. written by an earlier `jet start
    /// --persist`. Use this to stop a kernel that wasn't tracked
    /// as a jet session. Mutually exclusive with the positional
    /// `session_id`.
    #[arg(long)]
    pub connection_file: Option<PathBuf>,

    /// A name used to identify the client shutting down the kernel.
    #[arg(long, env = "JET_SESSION_NAME")]
    pub session_name: Option<String>,

    #[command(flatten)]
    pub global: GlobalArgs,
}
