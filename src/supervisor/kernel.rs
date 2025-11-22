/*
 * kernel.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use rand::Rng;
use std::fmt::Display;
use std::path::PathBuf;
use std::process;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant, SystemTime};

use crate::callback_output::KernelResponse;
use crate::connection::connection::JupyterChannels;
use crate::connection::heartbeat::Heartbeat;
use crate::connection::iopub::Iopub;
use crate::error::Error;
use crate::kernel::kernel_spec::{InterruptMode, KernelSpec};
use crate::kernel::startup_method::StartupMethod;
use crate::msg::wire::jupyter_message::{Message, Status};
use crate::msg::wire::message_id::Id;
use crate::supervisor::broker::Broker;
use crate::supervisor::kernel_comm::KernelComm;
use crate::supervisor::kernel_info::KernelInfo;
use crate::supervisor::reply_receivers::ReplyReceivers;

pub struct Kernel {
    pub id: Id,
    pub info: KernelInfo,
    pub process: Mutex<process::Child>,
    pub comm: KernelComm,
}

impl Display for Kernel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'{}", self.info.spec.display_name, self.id)
    }
}

impl Kernel {
    pub fn start(spec_path: PathBuf, spec: KernelSpec) -> anyhow::Result<Self> {
        log::info!("Using kernel '{}'", spec.display_name);

        let kernel_id = Id::new();
        let mut cf_path = std::env::temp_dir();
        cf_path.push(format!("jet_connection_file_{}.json", kernel_id.as_str()));
        let kernel_cmd = spec.build_command(&cf_path);

        let (jupyter_channels, process) = match spec.get_connection_method() {
            StartupMethod::RegistrationFile => {
                JupyterChannels::init_with_registration_file(kernel_cmd, cf_path)?
            }
            StartupMethod::ConnectionFile => {
                JupyterChannels::init_with_connection_file(kernel_cmd, cf_path)?
            }
        };

        log::info!(
            "Kernel '{}' started with id {}",
            spec.display_name,
            kernel_id
        );

        let iopub_broker = Arc::new(Broker::new(format!("IOPub{kernel_id}")));
        let shell_broker = Arc::new(Broker::new(format!("Shell{kernel_id}")));
        let stdin_broker = Arc::new(Broker::new(format!("Stdin{kernel_id}")));
        let control_broker = Arc::new(Broker::new(format!("Control{kernel_id}")));

        let (stopper, monitor) = Self::loop_heartbeat(jupyter_channels.heartbeat);
        let iopub_tx = Self::listen_iopub(jupyter_channels.iopub, Arc::clone(&iopub_broker));

        let kernel_comm = KernelComm {
            session: jupyter_channels.session.clone(),
            heartbeat_stopper: stopper,
            heartbeat_monitor: Mutex::new(monitor),
            iopub_stopper: iopub_tx,
            shell_channel: Mutex::new(jupyter_channels.shell),
            stdin_channel: Mutex::new(jupyter_channels.stdin),
            control_channel: Mutex::new(jupyter_channels.control),
            iopub_broker,
            shell_broker,
            stdin_broker,
            control_broker,
        };

        let kernel_info_reply = kernel_comm.subscribe()?;

        Ok(Self {
            id: kernel_id,
            comm: kernel_comm,
            process: Mutex::new(process),
            info: KernelInfo {
                spec_path,
                spec: spec,
                info: kernel_info_reply,
                start_time: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)?
                    .as_secs() as u64,
            },
        })
    }

    /// Spawn a thread that continuously receives IOPub messages and routes them through the broker
    fn listen_iopub(iopub: Iopub, broker: Arc<Broker>) -> Sender<StopIopub> {
        let (stop_tx, stop_rx) = channel();

        spawn(move || {
            log::info!("IOPub thread started");

            let cleanup_interval = broker.config.cleanup_interval;
            let mut last_cleanup = Instant::now();

            loop {
                if stop_rx.try_recv().is_ok() {
                    log::trace!("Quitting iopub thread");
                    return;
                }

                // Receive with a short timeout to allow periodic cleanup
                if let Some(msg) = iopub.recv_with_timeout(100) {
                    log::trace!("Message received on iopub: {}", msg.describe(),);
                    broker.route(msg);
                };

                // Periodic cleanup of stale requests and orphan messages
                if last_cleanup.elapsed() >= cleanup_interval {
                    broker.purge();
                    broker.log_stats();
                    last_cleanup = Instant::now();
                }
            }
        });

        stop_tx
    }

    /// Spawn a thread that periodically send heartbeat messages and verify replies
    fn loop_heartbeat(heartbeat: Heartbeat) -> (Sender<StopHeartbeat>, Receiver<HeartbeatFailed>) {
        let (stop_tx, stop_rx) = channel();
        let (failed_tx, failed_rx) = channel();

        spawn(move || {
            log::info!("Heartbeat thread started");

            loop {
                if stop_rx.try_recv().is_ok() {
                    log::trace!("Quitting heartbeat thread");
                    return;
                }

                let mut rng = rand::rng();
                // We just send some random number to the kernel
                let bytes: Vec<u8> = (0..32).map(|_| rng.random_range(0..10)).collect();

                heartbeat.send(zmq::Message::from(&bytes));

                // Then we (hopefully) wait to receive the same message back
                if let Ok(reply) = heartbeat.recv_with_timeout(1000) {
                    let reply_slice: &[u8] = reply.as_ref();

                    if reply_slice != bytes.as_slice() {
                        log::warn!(
                            "Heartbeat reply not the same as request: {:?} != {:?}",
                            bytes,
                            reply_slice,
                        )
                    }
                } else {
                    log::error!("Heartbeat timed out");
                    failed_tx.send(HeartbeatFailed).unwrap();
                }

                sleep(Duration::from_millis(500));
            }
        });

        (stop_tx, failed_rx)
    }

    pub fn shutdown(&self) -> anyhow::Result<()> {
        log::info!("Shutting down kernel {self}");

        match self.comm.request_shutdown() {
            Ok(Message::ShutdownReply(msg)) if msg.content.status == Status::Error => {
                log::error!("{self} reported an error during shutdown");
            }
            Ok(Message::ShutdownReply(msg)) if msg.content.status == Status::Ok => {
                log::trace!("{self} reported successful shutdown");
            }
            Ok(_) => {
                log::warn!("{self} received unexpected reply to shutdown request");
            }
            Err(e) => {
                log::error!("Failed to request shutdown for {self}: {e}");
            }
        };

        let mut process = self.process.lock().unwrap();
        match process.try_wait()? {
            Some(status) => {
                log::info!("{self} exited with status {status} after shutdown request");
            }
            None => {
                log::warn!("{self} did not exit in time, killing process");
                process.kill()?;
            }
        }

        Ok(())
    }

    /// Send an interrupt reply using a signal or message, depending on what the kernel supports
    ///
    /// Returns receivers if the interrupt was sent as a message.
    pub fn request_interrupt(&self) -> Result<Option<ReplyReceivers>, Error> {
        let interrupt_mode = match self.info.spec.interrupt_mode {
            Some(ref mode) => mode.clone(),
            None => match &self.info.info.protocol_version {
                // This is technically off-label; the Jupyter protocol says that if kernels don't
                // indicate in the spec that they support message interrupts then clients should
                // use a signal. However, right now neither Ipykernel nor Ark indicate they support
                // message interrupts, but they do. So unless I'm misinterpreting the protocol,
                // this is the informal standard that kernels tend to use.
                // https://jupyter-client.readthedocs.io/en/latest/messaging.html#kernel-interrupt
                Some(version) if version >= &String::from("5.3") => InterruptMode::Message,
                _ => InterruptMode::Signal,
            },
        };

        match interrupt_mode {
            InterruptMode::Message => return self.comm.send_interrupt_request().map(Some),
            InterruptMode::Signal => {
                #[cfg(unix)]
                {
                    self.interrupt_signal()?;
                    return Ok(None);
                }
                #[cfg(not(unix))]
                return Err(Error::UnsupportedPlatform(
                    "Can't send interrupt using OS signal",
                ));
            }
        }
    }

    /// Pairs with `KernelComm.interrupt_message()`
    fn interrupt_signal(&self) -> Result<(), Error> {
        use nix::sys::signal::{self, Signal};
        use nix::unistd::Pid;

        let process_id = Pid::from_raw(self.process.lock().unwrap().id() as i32);

        if let Err(e) = signal::kill(process_id, Signal::SIGINT) {
            return Err(Error::Anyhow(anyhow::anyhow!(
                "Failed to interrupt using SIGINT: {e}"
            )));
        };

        Ok(())
    }

    pub fn recv_interrupt_reply(&self, receivers: &Option<ReplyReceivers>) -> KernelResponse {
        match receivers {
            Some(receivers) => self.comm.recv_interrupt_reply(&receivers),
            None => KernelResponse::Idle,
        }
    }
}

pub struct StopHeartbeat;
pub struct HeartbeatFailed;
pub struct StopIopub;
