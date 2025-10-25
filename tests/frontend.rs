use carpo::frontend::frontend::{ExecuteRequestOptions, Frontend};
use carpo::kernel::kernel_spec::KernelSpecFull;
use carpo::kernel::startup_method::StartupMethod;

use carpo::frontend::frontend;
use carpo::msg::wire::jupyter_message::Message;
use carpo::msg::wire::status::ExecutionState;

fn get_frontend(kernel: String) -> anyhow::Result<Frontend> {
    let selected_kernel = KernelSpecFull::get_all()
        .into_iter()
        .filter_map(|x| x.spec.ok())
        .filter(|x| x.display_name == kernel)
        .nth(0);

    let spec = match selected_kernel {
        Some(kernel) => kernel,
        None => panic!("No kernel found with name '{}'", kernel),
    };

    log::info!("Using kernel '{}'", spec.display_name);

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Get the startup command
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let connection_file_path = String::from(format!("connection_file_{}.json", kernel));
    let kernel_cmd = spec.build_command(&connection_file_path);

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Start the frontend
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let frontend = match spec.get_startup_method() {
        StartupMethod::RegistrationFile => {
            println!("Starting with registration file");
            Frontend::start_with_registration_file(kernel_cmd, connection_file_path.into())
        }
        StartupMethod::ConnectionFile => {
            println!("Starting with connection file");
            Frontend::start_with_connection_file(kernel_cmd, connection_file_path.into())
        }
    };

    let _kernel_info = frontend.subscribe();

    Ok(frontend)
}

#[test]
fn test_ark_with_registration_file() {
    let frontend = get_frontend(String::from("Ark R Kernel")).unwrap();

    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        loop {
            match frontend.iopub.recv() {
                Message::Stream(msg) => {
                    println!("Stream ({:#?}): {}", msg.content.name, msg.content.text)
                }
                msg @ Message::Status(_) => tx.send(msg).unwrap(),
                msg @ Message::ExecuteInput(_) => tx.send(msg).unwrap(),
                msg @ Message::ExecuteResult(_) => tx.send(msg).unwrap(),
                msg @ Message::ExecuteReply(_) => tx.send(msg).unwrap(),
                _ => todo!(),
            };
        }
    });

    let code = "1 + 1";
    let msg_id = frontend
        .shell
        .send_execute_request(code, ExecuteRequestOptions::default());

    let mut received_input = false;
    let mut received_result = false;

    loop {
        match rx.recv().unwrap() {
            Message::ExecuteInput(msg) => {
                assert_eq!(code, msg.content.code);
                received_input = true;
            }
            Message::ExecuteResult(msg) => {
                assert_eq!("[1] 2", msg.content.data["text/plain"]);
                received_result = true;
            }
            Message::Status(msg) => match msg.content.execution_state {
                ExecutionState::Idle => break,
                other => println!("Received execution status {:#?}", other),
            },
            other => panic!("Received unexpected message {:#?}", other),
        };
    }

    assert!(received_input);
    assert!(received_result);

    let _ = frontend.shell.try_recv_execute_reply(&msg_id);
}

#[test]
fn test_ark_with_connection_file() {
    let frontend = get_frontend(String::from("Ark R Kernel (connection file method)")).unwrap();

    let code = "1 + 1";

    let msg_id = frontend
        .shell
        .send_execute_request(code, frontend::ExecuteRequestOptions::default());
    frontend.iopub.recv_busy();

    let input = frontend.iopub.recv_execute_input();
    let reply = frontend.iopub.recv_execute_result();

    assert_eq!(code, input.code);
    assert_eq!("[1] 2", reply);

    frontend.iopub.recv_idle();
    let _ = frontend.shell.try_recv_execute_reply(&msg_id);
}

#[test]
fn test_ipykernel() {
    let frontend = get_frontend(String::from("Python 3 (ipykernel)")).unwrap();

    let code = "1 + 1";

    let msg_id = frontend
        .shell
        .send_execute_request(code, frontend::ExecuteRequestOptions::default());
    frontend.iopub.recv_busy();

    let input = frontend.iopub.recv_execute_input();
    let reply = frontend.iopub.recv_execute_result();

    assert_eq!(code, input.code);
    assert_eq!("2", reply);

    frontend.iopub.recv_idle();
    let _ = frontend.shell.try_recv_execute_reply(&msg_id);
}
