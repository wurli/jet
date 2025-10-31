use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use anyhow::Result;

use crate::error::Error;
use crate::msg::wire::message_id::Id;
use crate::supervisor::kernel::Kernel;
use crate::supervisor::kernel::KernelInfo;

pub static KERNEL_MANAGER: OnceLock<KernelManager> = OnceLock::new();

pub struct KernelManager {
    kernels: RwLock<HashMap<String, Arc<Kernel>>>,
}

impl KernelManager {
    pub fn get() -> &'static Self {
        KERNEL_MANAGER.get_or_init(|| KernelManager::new())
    }

    fn new() -> Self {
        Self {
            kernels: RwLock::new(HashMap::new()),
        }
    }

    pub fn add_kernel(&self, id: Id, state: Kernel) -> Result<(), Error> {
        let mut kernels = self.kernels.write().unwrap();
        if kernels.contains_key(&String::from(id.clone())) {
            return Err(Error::KernelAlreadyRunning(id));
        }
        kernels.insert(String::from(id), Arc::new(state));
        Ok(())
    }

    pub fn get_kernel(&self, id: &Id) -> Result<Arc<Kernel>, Error> {
        let kernels = self.kernels.read().unwrap();
        if let Some(kernel) = kernels.get(&String::from(id.clone())) {
            Ok(Arc::clone(kernel))
        } else {
            Err(Error::KernelNotRunning(id.clone()))
        }
    }

    pub fn remove_kernel(&self, id: &Id) -> Result<(), Error> {
        let mut kernels = self.kernels.write().unwrap();
        let res = kernels.remove(&String::from(id.clone()));
        if let Some(_) = res {
            Ok(())
        } else {
            log::error!("Could not remove non-active kernel {}", id);
            Err(Error::KernelNotRunning(id.clone()))
        }
    }

    pub fn list_kernels(&self) -> HashMap<String, KernelInfo> {
        let kernels = self.kernels.read().unwrap();

        kernels
            .iter()
            .map(|(k, v)| (k.clone(), v.info.clone()))
            .collect()
    }

    pub fn kernel_exists(&self, id: &String) -> bool {
        let kernels = self.kernels.read().unwrap();
        kernels.contains_key(id)
    }
}
