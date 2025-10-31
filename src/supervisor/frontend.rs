use std::sync::{OnceLock, mpsc::channel};

use crate::{
    msg::wire::{
        jupyter_message::Message, message_id::Id, shutdown_request::ShutdownRequest,
        status::ExecutionState,
    },
    supervisor::kernel_manager::{KernelInfo, KernelManager},
};


pub struct Frontend {}

impl Frontend {

    pub fn request_shutdown(kernel_id: &Id) -> anyhow::Result<Message> {
        Self::request_shutdown_impl(&kernel_id, false)
    }

    pub fn request_restart(kernel_id: &Id) -> anyhow::Result<Message> {
        Self::request_shutdown_impl(&kernel_id, true)
    }

    /// This is a mess
    fn request_shutdown_impl(kernel_id: &Id, restart: bool) -> anyhow::Result<Message> {
        Self::kernel_manager()
            .with_kernel(&kernel_id, |kernel| {
                let request_id = {
                    let control = kernel.comm.control_channel.lock().unwrap();
                    let request_id = control.send(ShutdownRequest { restart });
                    request_id
                };
                log::info!("Sent shutdown_request {}", request_id);
                let (control_tx, control_rx) = channel();
                let (iopub_tx, iopub_rx) = channel();
                let (stdin_tx, stdin_rx) = channel();

                kernel.brokers.iopub.register_request(&request_id, iopub_tx);
                kernel.stdin_broker.register_request(&request_id, stdin_tx);
                kernel
                    .control_broker
                    .register_request(&request_id, control_tx);

                log::info!("Entering shutdown reply wait loop");
                loop {
                    match iopub_rx.try_recv() {
                        Ok(msg) => match msg {
                            Message::ShutdownReply(_) => {
                                log::info!("Received shutdown_reply on iopub (non-standard)");
                                return Ok(msg);
                            }
                            Message::Status(status_msg)
                                if status_msg.content.execution_state == ExecutionState::Idle =>
                            {
                                kernel
                                    .brokers
                                    .iopub
                                    .unregister_request(&request_id, "idle status received");
                                log::trace!("Received idle status");
                            }
                            _ => {
                                log::trace!(
                                    "Received unexpected message on iopub: {}",
                                    msg.describe()
                                )
                            }
                        },
                        Err(_) => {}
                    }

                    let _ = Self::recv_all_incoming_control(&kernel_id);

                    match stdin_rx.try_recv() {
                        Ok(msg @ Message::InputRequest(_)) => return Ok(msg),
                        Ok(msg) => log::warn!("Received unexpected reply {}", msg.describe()),
                        Err(_) => {}
                    }

                    let _ = Self::recv_all_incoming_control(&kernel_id);

                    match control_rx.try_recv() {
                        Ok(reply @ Message::ShutdownReply(_)) => {
                            log::info!("Received shutdown_reply on control (standard)");
                            kernel
                                .control_broker
                                .unregister_request(&request_id, "reply received");
                            return Ok(reply);
                        }
                        Ok(other) => {
                            log::warn!(
                                "Expected shutdown_reply but received unexpected message: {:#?}",
                                other
                            );
                            return Err(anyhow::anyhow!(
                                "Expected shutdown_reply but received unexpected message: {:#?}",
                                other
                            ));
                        }
                        Err(_) => {}
                    }

                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            })
            .unwrap()
    }

    // pub fn send_stdin<T: ProtocolMessage>(kernel_id: &Id, message: T) -> anyhow::Result<()> {
    //     let kernel = Self::kernel_manager().get_kernel(&kernel_id)?;
    //     let (msg, _request_id) = kernel.make_jupyter_message(message);
    //     kernel.send_stdin(msg);
    //     Ok(())
    // }
    //
    // pub fn is_request_active(kernel_id: &Id, request_id: &Id) -> anyhow::Result<bool> {
    //     Self::kernel_manager().with_kernel(&kernel_id, |kernel| {
    //         kernel.shell_broker.is_active(request_id)
    //             | kernel.brokers.iopub.is_active(request_id)
    //             | kernel.stdin_broker.is_active(request_id)
    //     })
    // }
}
