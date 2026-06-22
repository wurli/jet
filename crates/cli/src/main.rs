// jet — a runtimed-backed REPL for Jupyter kernels with kitty graphics.
//
// Spawns or attaches to a Jupyter kernel, drives a line-oriented REPL over
// the four ZMQ channels (shell, iopub, stdin, control), and renders PNG
// outputs inline using the kitty graphics protocol.

use anyhow::Result;
use clap::Parser;

mod cli;
mod commands;
mod picker;
mod pickers;
mod render;
mod repl;

use cli::{Args, Command};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::Connect(c) => commands::run_connect(c).await,
        Command::Attach(c) => commands::run_attach(c).await,
        Command::ListSessions(c) => commands::run_list(c).await,
        Command::ListKernels(c) => commands::run_list_kernels(c),
    }
}
