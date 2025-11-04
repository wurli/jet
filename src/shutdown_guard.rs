use crate::supervisor::kernel_manager::KernelManager;

/// A guard that shuts down all running kernels when it goes out of scope. Intended to be tied to
/// the Lua state so that all kernels are shut down when Neovim exits.
pub struct ShutdownGuard;

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        log::trace!("Shutting down all kernels using guard");
        KernelManager::shutdown_all();
    }
}
