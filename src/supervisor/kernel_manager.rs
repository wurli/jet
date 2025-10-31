use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use anyhow::Result;

use crate::error::Error;
use crate::msg::wire::message_id::Id;
use crate::supervisor::kernel::Kernel;
use crate::supervisor::kernel_info::KernelInfo;

pub static KERNEL_MANAGER: OnceLock<KernelManager> = OnceLock::new();

pub struct KernelManager {
    kernels: RwLock<HashMap<String, Arc<Kernel>>>,
}

impl KernelManager {
    pub fn manager() -> &'static Self {
        KERNEL_MANAGER.get_or_init(|| KernelManager::new())
    }

    fn new() -> Self {
        Self {
            kernels: RwLock::new(HashMap::new()),
        }
    }

    pub fn add(kernel: Kernel) -> Result<(), Error> {
        let mut kernels = Self::manager().kernels.write().unwrap();
        if kernels.contains_key(&String::from(kernel.id.clone())) {
            log::warn!("Failed to add existing kernel {}", kernel);
            return Err(Error::KernelAlreadyRunning(kernel.id.clone()));
        }
        log::trace!("Failed to add existing kernel {}", kernel);
        kernels.insert(String::from(kernel.id.clone()), Arc::new(kernel));
        Ok(())
    }

    pub fn get(id: &Id) -> Result<Arc<Kernel>, Error> {
        let kernels = Self::manager().kernels.read().unwrap();
        if let Some(kernel) = kernels.get(&String::from(id.clone())) {
            Ok(Arc::clone(kernel))
        } else {
            Err(Error::KernelNotRunning(id.clone()))
        }
    }

    pub fn remove(id: &Id) -> Result<(), Error> {
        let mut kernels = Self::manager().kernels.write().unwrap();
        // TODO: check that the kernel is not active
        let res = kernels.remove(&String::from(id.clone()));
        if let Some(_) = res {
            Ok(())
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
        kernels.contains_key(&String::from(id.clone()))
    }
}
