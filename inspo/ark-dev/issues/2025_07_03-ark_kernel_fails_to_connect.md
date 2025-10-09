# Ark Kernel Fails to Connect

> <https://github.com/posit-dev/ark/issues/862>
> 
> * Author: @jjoeldaniel
> * State: OPEN
> * Labels: 

I am encountering an issue where the Ark kernel fails to establish a connection within a Windows-based Docker container running JupyterLab. I am able to successfully run R code using IRkernel in the same environment, but Ark consistently exhibits connection failures.

## Environment

Docker Version: 28.1.1
Base Image: mcr.microsoft.com/windows/server:ltsc2022

JupyterLab Version: 4.4.3
Conda Version: py311_25.5.1-0

R Version: 4.5.1
Ark Version: 0.1.195
IRkernel Version: 1.3.2




## Logs



```
[W 2025-06-30 15:49:02.525 ServerApp] Nudge: attempt 120 on kernel 339b791d-abc8-4cfa-8db3-4e1aab3d613d
[E 2025-06-30 15:49:02.844 ServerApp] Uncaught exception GET /api/kernels/339b791d-abc8-4cfa-8db3-4e1aab3d613d/channels?session_id=233900e2-03a1-47d3-aae5-f4bc8ab0630b (10.14.52.63)
    HTTPServerRequest(protocol='http', host='<host>:1234', method='GET', uri='/api/kernels/339b791d-abc8-4cfa-8db3-4e1aab3d613d/channels?session_id=233900e2-03a1-47d3-aae5-f4bc8ab0630b', version='HTTP/1.1', remote_ip='10.14.52.63')
    Traceback (most recent call last):
      File "C:\Miniconda3\Lib\site-packages\tornado\websocket.py", line 967, in _accept_connection
        await open_result
      File "C:\Miniconda3\Lib\site-packages\jupyter_server\services\kernels\websocket.py", line 75, in open
        await self.connection.connect()
    TimeoutError: Timeout
[W 2025-06-30 15:49:02.868 ServerApp] Timeout waiting for kernel_info reply from 339b791d-abc8-4cfa-8db3-4e1aab3d613d
[I 2025-06-30 15:49:02.869 ServerApp] Connecting to kernel 339b791d-abc8-4cfa-8db3-4e1aab3d613d.
  2025-06-30T22:49:02.874131Z ERROR  Received subscription message, but no `subscription_tx` is available to confirm on. Have we already received a subscription message once before?
    at crates\amalthea\src\socket\iopub.rs:283

[W 2025-06-30 15:49:02.913 ServerApp] Replacing stale connection: 339b791d-abc8-4cfa-8db3-4e1aab3d613d:233900e2-03a1-47d3-aae5-f4bc8ab0630b
[W 2025-06-30 15:49:07.414 ServerApp] Nudge: attempt 10 on kernel 339b791d-abc8-4cfa-8db3-4e1aab3d613d

... repeats
```


## Steps to Reproduce

```Dockerfile
FROM mcr.microsoft.com/windows/server:ltsc2022

# Install miniconda
RUN powershell Invoke-WebRequest -Uri https://repo.anaconda.com/miniconda/Miniconda3-py311_25.5.1-0-Windows-x86_64.exe -OutFile MinicondaInstaller.exe
RUN powershell Start-Process -FilePath MinicondaInstaller.exe -ArgumentList '/InstallationType=AllUsers','/RegisterPython=0','/S','/D=C:\Miniconda3' -NoNewWindow -Wait
RUN powershell Remove-Item -Force MinicondaInstaller.exe
RUN powershell -Command "[Environment]::SetEnvironmentVariable('Path', $env:Path + ';C:\Miniconda3;C:\Miniconda3\Scripts;C:\Miniconda3\condabin', 'Machine')"

RUN conda install jupyterlab -y
RUN ipython profile create

# Install R
RUN powershell Invoke-WebRequest -OutFile R-4.5.1-win.exe https://cran.r-project.org/bin/windows/base/R-4.5.1-win.exe
RUN R-4.5.1-win.exe /SILENT /DIR="C:\Program Files\R\R-4.5.1" && powershell Remove-Item "R-4.5.1-win.exe"
RUN powershell -Command "[Environment]::SetEnvironmentVariable('Path', $env:Path + ';C:\Program Files\R\R-4.5.1\bin', 'User')"
RUN powershell -Command "[Environment]::SetEnvironmentVariable('R_HOME', 'C:\Program Files\R\R-4.5.1', 'User')"

# Install Ark
RUN powershell Invoke-WebRequest -OutFile ark.zip https://github.com/posit-dev/ark/releases/download/0.1.195/ark-0.1.195-windows-x64.zip
RUN powershell Expand-Archive -Path "ark.zip" -DestinationPath "C:\ark" && powershell Remove-Item "ark.zip"
RUN powershell -Command "[Environment]::SetEnvironmentVariable('Path', $env:Path + ';C:\ark', 'User')"
RUN ark --install

# Install IRkernel
RUN Rscript --no-save --no-restore -e "install.packages('IRkernel', repos='https://cloud.r-project.org'); IRkernel::installspec()"

ENTRYPOINT ["jupyter", "lab", "--ip=0.0.0.0", "--port=1234", "--allow-root"]
```

1. Build Image
```Dockerfile
docker build -t <name> Dockerfile .
```

2. Run container
```Dockerfile
docker run --rm -p 1234:1234 -it <name>
```

3. Open URL and select Ark kernel.

4. Attempt to run any R code.

cc @josiahparry

## @JosiahParry at 2025-07-03T15:03:21Z

Please let us know if there is additional debugging we can do on our side to remediate this. 

We'd love for both our Linux and Windows images to both use Ark. 

## @lionel- at 2025-07-03T19:47:58Z

Could you run with the envvar `RUST_LOG` set to `"trace"` please, to get more complete logs.

## @lionel- at 2025-07-03T19:48:50Z

you'll need to set it in the `kernel.json` file installed by Ark as there's an env entry for `RUST_LOG` in that file. You can find the path e.g. via `jupyter kernel-spec list`.

## @jjoeldaniel at 2025-07-03T21:48:30Z

Here is the log from entrypoint until timeout after setting `RUST_LOG` to `"trace"`:

<details>

<summary> Error Log </summary>

```
[I 2025-07-03 14:29:22.559 ServerApp] Creating new notebook in
[I 2025-07-03 14:29:22.626 ServerApp] Writing notebook-signing key to C:\Users\ContainerAdministrator\AppData\Roaming\jupyter\notebook_secret
[I 2025-07-03 14:29:23.729 ServerApp] Kernel started: df060e8a-8fad-4c5f-82ec-1c6941429540
  2025-07-03T21:29:23.749495Z  INFO  Loaded connection information from frontend in C:\Users\ContainerAdministrator\AppData\Roaming\jupyter\runtime\kernel-df060e8a-8fad-4c5f-82ec-1c6941429540.json
    at crates\amalthea\src\kernel.rs:319

  2025-07-03T21:29:23.750250Z  INFO  Connection data: ConnectionFile { control_port: 49302, shell_port: 49298, stdin_port: 49300, iopub_port: 49299, hb_port: 49301, transport: "tcp", signature_scheme: "hmac-sha256", ip: "127.0.0.1", key: "a7edf568-77faa890704fa65ff1f67848" }
    at crates\amalthea\src\kernel.rs:320

  2025-07-03T21:29:23.777911Z TRACE  Binding to ZeroMQ 'Shell' socket at tcp://127.0.0.1:49298
    at crates\amalthea\src\socket\socket.rs:100

  2025-07-03T21:29:23.808755Z TRACE  Binding to ZeroMQ 'IOPub' socket at tcp://127.0.0.1:49299
    at crates\amalthea\src\socket\socket.rs:100

  2025-07-03T21:29:23.815211Z TRACE  Waiting for shell messages
    at crates\amalthea\src\socket\shell.rs:107

  2025-07-03T21:29:23.823079Z TRACE  Binding to ZeroMQ 'Heartbeat' socket at tcp://127.0.0.1:49301
    at crates\amalthea\src\socket\socket.rs:100

  2025-07-03T21:29:23.829781Z TRACE  Listening for heartbeats
    at crates\amalthea\src\socket\heartbeat.rs:30

  2025-07-03T21:29:23.831975Z TRACE  Binding to ZeroMQ 'Stdin' socket at tcp://127.0.0.1:49300
    at crates\amalthea\src\socket\socket.rs:100

  2025-07-03T21:29:23.842179Z TRACE  Binding to ZeroMQ 'Control' socket at tcp://127.0.0.1:49302
    at crates\amalthea\src\socket\socket.rs:100

  2025-07-03T21:29:23.859957Z TRACE  Binding to ZeroMQ 'OutboundNotifierTx' socket at inproc://outbound_notif
    at crates\amalthea\src\socket\socket.rs:133

  2025-07-03T21:29:23.864582Z TRACE  Connecting to ZeroMQ 'OutboundNotifierRx' socket at inproc://outbound_notif
    at crates\amalthea\src\socket\socket.rs:138

  2025-07-03T21:29:23.865735Z  INFO  Waiting on IOPub subscription confirmation
    at crates\amalthea\src\kernel.rs:288

  2025-07-03T21:29:23.867349Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:29:24.260931Z  INFO  Received subscribe message on IOPub with subscription ''.
    at crates\amalthea\src\socket\iopub.rs:265

  2025-07-03T21:29:24.261376Z  INFO  Sending `Welcome` message, `Starting` status, and subscription confirmation
    at crates\amalthea\src\socket\iopub.rs:287

  2025-07-03T21:29:24.261896Z  INFO  Received IOPub subscription confirmation, completing kernel connection
    at crates\amalthea\src\kernel.rs:291

  2025-07-03T21:29:24.262108Z TRACE  Sending 'iopub_welcome' message via IOPub socket
    at crates\amalthea\src\wire\wire_message.rs:204

  2025-07-03T21:29:24.262916Z TRACE  Sending 'status/starting' message via IOPub socket
    at crates\amalthea\src\wire\wire_message.rs:204

  2025-07-03T21:29:24.295537Z  INFO  Received shell request: JupyterMessage { zmq_identities: [[0, 128, 0, 0, 41]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_0", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:29:23.783880Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest }
    at crates\amalthea\src\socket\shell.rs:186

  2025-07-03T21:29:24.295673Z TRACE  Sending 'status/busy' message (reply to 'kernel_info_request') via IOPub socket
    at crates\amalthea\src\wire\wire_message.rs:196

  2025-07-03T21:29:24.296921Z TRACE  Got kernel info request; waiting for R to complete initialization
    at crates\ark\src\shell.rs:126

  2025-07-03T21:29:24.831409Z  INFO  Successfully opened R shared library at 'C:\Program Files\R\R-4.5.1\bin\x64\R.dll'.
    at crates\harp\src\library.rs:30

  2025-07-03T21:29:24.832024Z  INFO  Successfully opened R shared library at 'C:\Program Files\R\R-4.5.1\bin\x64\Rgraphapp.dll'.
    at crates\harp\src\library.rs:30

  2025-07-03T21:29:24.834562Z  INFO  Successfully opened R shared library at 'C:\Program Files\R\R-4.5.1\bin\x64\Rlapack.dll'.
    at crates\harp\src\library.rs:30

  2025-07-03T21:29:24.834919Z  INFO  Successfully opened R shared library at 'C:\Program Files\R\R-4.5.1\bin\x64\Riconv.dll'.
    at crates\harp\src\library.rs:30

  2025-07-03T21:29:24.835544Z  INFO  Successfully opened R shared library at 'C:\Program Files\R\R-4.5.1\bin\x64\Rblas.dll'.
    at crates\harp\src\library.rs:30

[W 2025-07-03 14:30:23.783 ServerApp] Timeout waiting for kernel_info reply from df060e8a-8fad-4c5f-82ec-1c6941429540
[I 2025-07-03 14:30:23.784 ServerApp] Connecting to kernel df060e8a-8fad-4c5f-82ec-1c6941429540.
  2025-07-03T21:30:23.790116Z  INFO  Received subscribe message on IOPub with subscription ''.
    at crates\amalthea\src\socket\iopub.rs:265

  2025-07-03T21:30:23.790568Z ERROR  Received subscription message, but no `subscription_tx` is available to confirm on. Have we already received a subscription message once before?
    at crates\amalthea\src\socket\iopub.rs:283

  2025-07-03T21:30:23.790905Z  WARN  Error processing inbound iopub message: Anyhow(Received subscription message, but no `subscription_tx` is available to confirm on. Have we already received a subscription message once before?)
    at crates\amalthea\src\socket\iopub.rs:156

[W 2025-07-03 14:30:23.803 ServerApp] The websocket_ping_timeout (90000) cannot be longer than the websocket_ping_interval (30000).
    Setting websocket_ping_timeout=30000
  2025-07-03T21:30:23.805132Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_2", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:23.804666Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:23.805635Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:24.313950Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_4", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:24.313327Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:24.314515Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:24.814778Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_6", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:24.813655Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:24.815345Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:25.316505Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_8", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:25.315492Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:25.317155Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:25.818041Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_10", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:25.816960Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:25.818620Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:26.318912Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_12", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:26.317616Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:26.319482Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:26.824076Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_14", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:26.822989Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:26.824671Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:27.325749Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_16", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:27.324466Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:27.326349Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:27.827271Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_18", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:27.825886Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:27.827885Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

[W 2025-07-03 14:30:28.326 ServerApp] Nudge: attempt 10 on kernel df060e8a-8fad-4c5f-82ec-1c6941429540
  2025-07-03T21:30:28.328708Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_20", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:28.327657Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:28.329288Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:28.830019Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_22", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:28.829270Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:28.830928Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:29.331557Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_24", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:29.330305Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:29.332318Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:29.832283Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_26", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:29.831082Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:29.832846Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:30.332566Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_28", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:30.331542Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:30.333185Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:30.834064Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_30", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:30.832761Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:30.834619Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:31.334778Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_32", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:31.333620Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:31.335489Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:31.833129Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_34", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:31.831557Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:31.833902Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:32.334119Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_36", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:32.333087Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:32.334988Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:30:32.842704Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_38", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:30:32.841510Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:30:32.843261Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

...

[W 2025-07-03 14:31:23.435 ServerApp] Nudge: attempt 120 on kernel df060e8a-8fad-4c5f-82ec-1c6941429540
  2025-07-03T21:31:23.436758Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 215]], header: JupyterHeader { msg_id: "34a426e2-607a-44f8-80cb-cb38c1c29e9f_18816_240", session: "34a426e2-607a-44f8-80cb-cb38c1c29e9f", username: "username", date: "2025-07-03T21:31:23.435774Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:31:23.437228Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

[W 2025-07-03 14:31:23.807 ServerApp] Timeout waiting for kernel_info reply from df060e8a-8fad-4c5f-82ec-1c6941429540
[I 2025-07-03 14:31:23.808 ServerApp] Connecting to kernel df060e8a-8fad-4c5f-82ec-1c6941429540.
  2025-07-03T21:31:23.813523Z  INFO  Received subscribe message on IOPub with subscription ''.
    at crates\amalthea\src\socket\iopub.rs:265

  2025-07-03T21:31:23.813985Z ERROR  Received subscription message, but no `subscription_tx` is available to confirm on. Have we already received a subscription message once before?
    at crates\amalthea\src\socket\iopub.rs:283

  2025-07-03T21:31:23.814404Z  WARN  Error processing inbound iopub message: Anyhow(Received subscription message, but no `subscription_tx` is available to confirm on. Have we already received a subscription message once before?)
    at crates\amalthea\src\socket\iopub.rs:156

[E 2025-07-03 14:31:23.828 ServerApp] Uncaught exception GET /api/kernels/df060e8a-8fad-4c5f-82ec-1c6941429540/channels?session_id=34a426e2-607a-44f8-80cb-cb38c1c29e9f (10.14.52.63)
    HTTPServerRequest(protocol='http', host='devet00161.esri.com:1234', method='GET', uri='/api/kernels/df060e8a-8fad-4c5f-82ec-1c6941429540/channels?session_id=34a426e2-607a-44f8-80cb-cb38c1c29e9f', version='HTTP/1.1', remote_ip='10.14.52.63')
    Traceback (most recent call last):
      File "C:\Miniconda3\Lib\site-packages\tornado\websocket.py", line 967, in _accept_connection
        await open_result
      File "C:\Miniconda3\Lib\site-packages\jupyter_server\services\kernels\websocket.py", line 75, in open
        await self.connection.connect()
    TimeoutError: Timeout
  2025-07-03T21:31:23.840673Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 216]], header: JupyterHeader { msg_id: "321793aa-04f5-4f2f-864e-33ef752b921b_18816_1", session: "321793aa-04f5-4f2f-864e-33ef752b921b", username: "username", date: "2025-07-03T21:31:23.839163Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

  2025-07-03T21:31:23.841139Z TRACE  Waiting for control messages
    at crates\amalthea\src\socket\control.rs:58

  2025-07-03T21:31:24.344786Z  WARN  Could not handle control message: Unsupported message received on 'control': KernelInfoRequest(JupyterMessage { zmq_identities: [[0, 0, 0, 44, 216]], header: JupyterHeader { msg_id: "321793aa-04f5-4f2f-864e-33ef752b921b_18816_3", session: "321793aa-04f5-4f2f-864e-33ef752b921b", username: "username", date: "2025-07-03T21:31:24.343450Z", msg_type: "kernel_info_request", version: "5.3" }, parent_header: None, content: KernelInfoRequest })
    at crates\amalthea\src\socket\control.rs:69

... repeats ...
```

</details>