/*
 * kernel_manager.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock, RwLock};

use anyhow::Result;

use crate::error::Error;
use crate::kernel::kernel_spec::KernelSpec;
use crate::msg::wire::message_id::Id;
use crate::supervisor::kernel::Kernel;
use crate::supervisor::kernel_info::KernelInfo;

pub static KERNEL_MANAGER: OnceLock<KernelManager> = OnceLock::new();

pub struct KernelManager {
    kernels: RwLock<HashMap<String, Arc<Kernel>>>,
}

impl KernelManager {
    pub fn manager() -> &'static Self {
        KERNEL_MANAGER.get_or_init(KernelManager::new)
    }

    fn new() -> Self {
        Self {
            kernels: RwLock::new(HashMap::new()),
        }
    }

    pub fn start(spec_path: PathBuf) -> anyhow::Result<(Id, KernelInfo)> {
        let spec = KernelSpec::from_file(&spec_path)?;
        let kernel = Kernel::start(spec_path, spec)?;
        let out = (kernel.id.clone(), kernel.info.clone());
        Self::add(kernel)?;
        Ok(out)
    }

    pub fn add(kernel: Kernel) -> Result<(), Error> {
        let mut kernels = Self::manager().kernels.write().unwrap();
        if kernels.contains_key(kernel.id.as_ref()) {
            log::warn!("Failed to add existing kernel {}", kernel);
            return Err(Error::KernelAlreadyRunning(kernel.id));
        }
        log::trace!("Added new kernel {} to the manager", kernel);
        kernels.insert(String::from(kernel.id.clone()), Arc::new(kernel));
        Ok(())
    }

    pub fn get(id: &Id) -> Result<Arc<Kernel>, Error> {
        let kernels = Self::manager().kernels.read().unwrap();
        if let Some(kernel) = kernels.get(id.as_ref()) {
            Ok(Arc::clone(kernel))
        } else {
            Err(Error::KernelNotRunning(id.clone()))
        }
    }

    pub fn shutdown(id: &Id) -> anyhow::Result<()> {
        // Note, we probs don't _need_ the shutdown call since this happens on drop anyway, but
        // probs best to be explicit.
        Self::take(id)?.shutdown()
    }

    fn take(id: &Id) -> Result<Arc<Kernel>, Error> {
        let mut kernels = Self::manager().kernels.write().unwrap();
        if let Some(kernel) = kernels.remove(id.as_ref()) {
            Ok(kernel)
        } else {
            log::error!("Could not remove non-active kernel {}", id);
            Err(Error::KernelNotRunning(id.clone()))
        }
    }

    pub fn list() -> HashMap<String, KernelInfo> {
        let kernels = Self::manager().kernels.read().unwrap();

        kernels
            .iter()
            .map(|(k, v)| (k.clone(), v.info.clone()))
            .collect()
    }

    pub fn has(id: &Id) -> bool {
        let kernels = Self::manager().kernels.read().unwrap();
        kernels.contains_key(id.as_ref())
    }

    pub fn shutdown_all() {
        let kernels = Self::manager().kernels.write().unwrap();
        log::info!("Shutting down all {} registered kernels", kernels.len());

        for (_, kernel) in kernels.iter() {
            match kernel.shutdown() {
                Ok(()) => log::trace!("Successfully shut down kernel {kernel}"),
                Err(e) => log::error!("Failed to shut down kernel {kernel}: {e}"),
            }
        }
    }
}
