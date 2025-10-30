use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use serde::{Deserialize, Serialize};

use crate::msg::wire::language_info::LanguageInfo;
use crate::supervisor::broker::Broker;

pub type KernelId = String;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KernelInfo {
    pub spec_path: String,
    pub display_name: String,
    pub banner: String,
    pub language: LanguageInfo,
}

pub struct KernelState {
    pub id: KernelId,
    pub info: KernelInfo,
    pub connection: InputChannels,
    pub iopub_broker: Arc<Broker>,
    pub shell_broker: Arc<Broker>,
    pub stdin_broker: Arc<Broker>,
    pub control_broker: Arc<Broker>,
}

pub struct InputChannels {
    pub shell: Mutex<crate::connection::shell::Shell>,
    pub stdin: Mutex<crate::connection::stdin::Stdin>,
    pub control: Mutex<crate::connection::control::Control>,
}

pub struct KernelManager {
    kernels: RwLock<HashMap<KernelId, Arc<KernelState>>>,
}

impl KernelManager {
    pub fn new() -> Self {
        Self {
            kernels: RwLock::new(HashMap::new()),
        }
    }

    pub fn add_kernel(&self, id: KernelId, state: KernelState) -> anyhow::Result<()> {
        let mut kernels = self.kernels.write().unwrap();
        if kernels.contains_key(&id) {
            return Err(anyhow::anyhow!("Kernel with id '{}' already exists", id));
        }
        kernels.insert(id, Arc::new(state));
        Ok(())
    }

    pub fn get_kernel(&self, id: &KernelId) -> Option<Arc<KernelState>> {
        let kernels = self.kernels.read().unwrap();
        kernels.get(id).map(Arc::clone)
    }

    pub fn remove_kernel(&self, id: &KernelId) -> Option<Arc<KernelState>> {
        let mut kernels = self.kernels.write().unwrap();
        kernels.remove(id)
    }

    pub fn list_kernels(&self) -> HashMap<KernelId, KernelInfo> {
        let kernels = self.kernels.read().unwrap();

        kernels
            .iter()
            .map(|(k, v)| (k.clone(), v.info.clone()))
            .collect()
    }

    pub fn kernel_exists(&self, id: &KernelId) -> bool {
        let kernels = self.kernels.read().unwrap();
        kernels.contains_key(id)
    }

    /// Call `f()` on kernel `id`
    pub fn with_kernel<F, R>(&self, id: &KernelId, f: F) -> anyhow::Result<R>
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

impl Default for KernelManager {
    fn default() -> Self {
        Self::new()
    }
}
