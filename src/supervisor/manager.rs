use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::msg::wire::language_info::LanguageInfo;
use crate::msg::wire::message_id::Id;
use crate::supervisor::broker::Broker;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KernelInfo {
    /// The path to the kernel's spec file
    pub spec_path: String,
    /// The spec file's `display_name`
    pub display_name: String,
    /// The banner given by the kernel's `KernelInfoReply`
    pub banner: String,
    /// The language info given by the kernel's `KernelInfoReply`
    pub language: LanguageInfo,
}

pub struct KernelState {
    pub id: Id,
    pub info: KernelInfo,
    pub connection: InputChannels,
    pub iopub_broker: Arc<Broker>,
    pub shell_broker: Arc<Broker>,
    pub stdin_broker: Arc<Broker>,
    pub control_broker: Arc<Broker>,
}

/// These are the channels on which we might want to send data (as well as receive)
pub struct InputChannels {
    pub shell: Mutex<crate::connection::shell::Shell>,
    pub stdin: Mutex<crate::connection::stdin::Stdin>,
    pub control: Mutex<crate::connection::control::Control>,
}

pub struct KernelManager {
    kernels: RwLock<HashMap<String, Arc<KernelState>>>,
}

impl KernelManager {
    pub fn new() -> Self {
        Self {
            kernels: RwLock::new(HashMap::new()),
        }
    }

    pub fn add_kernel(&self, id: Id, state: KernelState) -> anyhow::Result<()> {
        let mut kernels = self.kernels.write().unwrap();
        if kernels.contains_key(&String::from(id.clone())) {
            return Err(anyhow::anyhow!("Kernel with id '{}' already exists", id));
        }
        kernels.insert(String::from(id), Arc::new(state));
        Ok(())
    }

    pub fn get_kernel(&self, id: &Id) -> Result<Arc<KernelState>, Error> {
        let kernels = self.kernels.read().unwrap();
        if let Some(kernel) = kernels.get(&String::from(id.clone())) {
            Ok(Arc::clone(kernel))
        } else {
            Err(Error::KernelNotRunning(id.clone()))
        }
    }

    pub fn remove_kernel(&self, id: Id) -> Option<Arc<KernelState>> {
        let mut kernels = self.kernels.write().unwrap();
        let res = kernels.remove(&String::from(id.clone()));
        if let None = res {
            log::warn!("Could not remove non-active kernel {}", id)
        };
        res
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

    /// Call `f()` on kernel `id`
    pub fn with_kernel<F, R>(&self, id: &String, f: F) -> anyhow::Result<R>
    where
        F: FnOnce(&KernelState) -> R,
    {
        let kernels = self.kernels.read().unwrap();
        kernels
            .get(id)
            .map(|k| f(k.as_ref()))
            .ok_or_else(|| anyhow::anyhow!("Kernel '{}' not found", id))
    }
}
