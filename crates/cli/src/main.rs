// jet — a runtimed-backed REPL for Jupyter kernels with kitty graphics.
//
// Spawns or attaches to a Jupyter kernel, drives a line-oriented REPL over
// the four ZMQ channels (shell, iopub, stdin, control), and renders PNG
// outputs inline using the kitty graphics protocol.

use anyhow::Result;
use clap::Parser;

mod cli;
mod commands;
mod completer;
mod fmt;
mod picker;
mod pickers;
mod render;
mod repl;

use cli::{Args, Command};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    if let Some(g) = args.command.global() {
        jet_core::logger::init_logger(g.log.as_deref());
    }
    match args.command {
        Command::Start(c) => commands::run_connect(c).await,
        Command::Attach(c) => commands::run_attach(c).await,
        Command::Stop(c) => commands::run_stop(c).await,
        Command::ListSessions(c) => commands::run_list_sessions(c).await,
        Command::ListKernels(c) => commands::run_list_kernels(c),
        Command::Execute(c) => commands::run_execute(c).await,
        Command::Send(c) => commands::run_send(c).await,
        Command::Show(c) => commands::run_show(c),
        Command::Skill => commands::run_skill(),
    }
}
