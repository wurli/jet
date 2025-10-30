use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use crate::{
    kernel::kernel_spec::KernelSpec,
    msg::wire::kernel_info_reply::KernelInfoReply,
    supervisor::broker::Broker,
};

pub type KernelId = String;

#[derive(Debug, Clone)]
pub struct KernelInfo {
    pub spec: KernelSpec,
    pub info: KernelInfoReply,
}

pub struct KernelState {
    pub id: KernelId,
    pub info: KernelInfo,
    pub connection: KernelConnection,
    pub iopub_broker: Arc<Broker>,
    pub shell_broker: Arc<Broker>,
    pub stdin_broker: Arc<Broker>,
}

pub struct KernelConnection {
    pub shell: Mutex<crate::connection::shell::Shell>,
    pub stdin: Mutex<crate::connection::stdin::Stdin>,
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

    pub fn list_kernels(&self) -> Vec<KernelId> {
        let kernels = self.kernels.read().unwrap();
        kernels.keys().cloned().collect()
    }

    pub fn kernel_exists(&self, id: &KernelId) -> bool {
        let kernels = self.kernels.read().unwrap();
        kernels.contains_key(id)
    }

    pub fn with_kernel<F, R>(&self, id: &KernelId, f: F) -> anyhow::Result<R>
    where
        F: FnOnce(&KernelState) -> R,
    {
        let kernels = self.kernels.read().unwrap();
        kernels
            .get(id)
            .map(|k| f(k.as_ref()))
            .ok_or_else(|| anyhow::anyhow!("Kernel with id '{}' not found", id))
    }
}

impl Default for KernelManager {
    fn default() -> Self {
        Self::new()
    }
}
