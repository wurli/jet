use std::path::PathBuf;

use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand};

use crate::kernel;

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

pub struct KernelSpec {
    pub language: String,
    pub argv: Vec<String>,
}

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
    /// Leave any kcserver this process spawned running after jet exits, so
    /// later invocations can reconnect to the same kernel.
    #[arg(long)]
    pub persist: bool,

    /// Kernel argv. Pass after `--`. The literal string `{connection_file}`
    /// is substituted by kallichore with the path to the generated Jupyter
    /// connection file; if your argv doesn't include it, jet appends
    /// `-f {connection_file}` for you.
    /// Example: jet connect --language r -- /path/to/ark --connection_file {connection_file} --session-mode console
    #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
    pub kernel: Vec<String>,

    /// Language label for the session (e.g. `python`, `r`).
    #[arg(long)]
    pub language: String,

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

impl ConnectArgs {
    pub fn kernel_spec(&self) -> KernelSpec {
        KernelSpec {
            language: self.language.clone(),
            argv: kernel::build_argv(&self.kernel),
        }
    }
}
