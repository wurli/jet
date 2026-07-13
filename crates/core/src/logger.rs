//! env_logger setup shared by the CLI and the Lua binding.

use std::path::Path;

/// Initialize `env_logger` to write to `log_file`, if given. Idempotent:
/// repeated calls (e.g. across both `jet` entry points or multiple
/// `require('jet')` loads) silently no-op after the first success.
pub fn init_logger(log_file: Option<&Path>) {
    let Some(path) = log_file else { return };
    let Ok(file) = std::fs::File::create(path) else {
        return;
    };
    let _ = env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(Box::new(file)))
        .try_init();
}
