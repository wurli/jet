use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "jet", about = "kallichore-backed REPL with kitty graphics")]
pub struct Args {
    /// Path to the kcserver binary.
    #[arg(long, default_value = "kcserver")]
    pub kcserver: String,

    /// Connect to an already-running kcserver instead of spawning one.
    /// Pass the path to its connection file.
    #[arg(long)]
    pub connect: Option<PathBuf>,

    /// Kernel argv. Pass after `--`. Use `{connection_file}` as the
    /// placeholder kallichore replaces with the generated connection file.
    /// Default starts an ipython kernel.
    /// Example: jet --language r -- /path/to/ark --connection_file {connection_file} --session-mode console
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub kernel: Vec<String>,

    /// Language label for the session.
    #[arg(long, default_value = "python")]
    pub language: String,

    /// Disable kitty graphics; PNGs are reported as `[image/png NxN bytes]`.
    #[arg(long)]
    pub no_graphics: bool,
}
