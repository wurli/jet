# Fix Jupyter connection race condition

> <https://github.com/posit-dev/ark/issues/563>
> 
> * Author: @lionel-
> * State: CLOSED
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABSPDwBw", name = "bug", description = "Something isn't working", color = "E99695"), list(id = "LA_kwDOJkuGPc8AAAABwXx3RQ", name = "area: jupyter kernel", description = "", color = "C2E0C6"), list(id = "LA_kwDOJkuGPc8AAAABwhpKcQ", name = "infra: tests", description = "", color = "bfdadc")

We're seeing issues in Windows CI that look like:

```
thread 'dummy_kernel' panicked at crates/ark/src/start.rs:112:9:
Couldn't connect to frontend: SocketBindError("Control", "tcp://127.0.0.1:23393", Address already in use)
```

I can also reproduce it locally by running integration tests in a loop. It's a mystery why it happens so often on the Windows CI though.

This might be due to the "classic jupyter race condition": https://github.com/jupyter/jupyter_client/issues/487. In Jupyter's connection scheme, the client searches for available ports, communicates those to the server which then tries to bind to them. This fails if any of the ports end up getting used up in the meantime.

There is no other solution than to let the kernel pick the ports. In the linked issue, they suggest implementing this scheme:

> The client opens a socket A, passes the port of this socket to the kernel that it launches and waits the kernel starts, finds free ports to bind shell, control, stdin, heartbeat and iopub sockets. Then it connects to the socket A of the client, sends a message containing these ports, and close the connection to socket A. Upon reception of this message, the client connects to the kernel and closes the socket A.

Essentially the client would pick a port for a handshake socket, bind to it, and send this connection info:

```json
{
  "transport": "tcp",
  "signature_scheme": "hmac-sha256",
  "ip": "127.0.0.1",
  "key": "a0436f6c-1916-498b-8eb9-e81ab9368e84",
  "handshake_port": 40885
}
```

And the server would connect to the handshake socket and send back:

```json
{
  "control_port": 50160,
  "shell_port": 57503,
  "stdin_port": 52597,
  "hb_port": 42540,
  "iopub_port": 40885,
}
```

On the server side, it looks like we can use `:*` or `:0` to let the OS pick a port: https://stackoverflow.com/questions/16699890/connect-to-first-free-port-with-tcp-using-0mq

Positron could also use this to make the initial connection to Ark more robust, cc @jmcphers.

## @jmcphers at 2024-10-03T15:08:43Z

I think this has also been an area of concern when running tests in parallel. This scheme should be implemented in the new kernel supervisor. We'd need some way for the kernel to advertise that it supports this mechanism (maybe a new field we pass when starting).

## @lionel- at 2024-10-03T17:11:42Z

> We'd need some way for the kernel to advertise that it supports this mechanism (maybe a new field we pass when starting).

In the linked thread someone suggests adding a field `handshake: true` to the kernelspec to indicate the connection file sent by the client may contain a handshake port to connect to.

## @DavisVaughan at 2024-10-03T21:06:17Z

See also this JEP that has been "Approved" and is therefore supposedly the recommended way to fix this problem https://github.com/jupyter/enhancement-proposals/pull/66

Nicely viewable at https://jupyter.org/enhancement-proposals/66-jupyter-handshaking/jupyter-handshaking.html

## @DavisVaughan at 2024-10-03T21:42:01Z

So it sounds like we'd add 1 new optional field to `ConnectionFile` called `handshake_port`:

```rust
pub struct ConnectionFile {
    /// ZeroMQ port: Handshake channel
    pub handshake_port: Option<u16>,

    /// ZeroMQ port: Control channel (kernel interrupts)
    pub control_port: u16,

    /// ZeroMQ port: Shell channel (execution, completion)
    pub shell_port: u16,

    /// ZeroMQ port: Standard input channel (prompts)
    pub stdin_port: u16,

    /// ZeroMQ port: IOPub channel (broadcasts input/output)
    pub iopub_port: u16,

    /// ZeroMQ port: Heartbeat messages (echo)
    pub hb_port: u16,

    /// The transport type to use for ZeroMQ; generally "tcp"
    pub transport: String,

    /// The signature scheme to use for messages; generally "hmac-sha256"
    pub signature_scheme: String,

    /// The IP address to bind to
    pub ip: String,

    /// The HMAC-256 signing key, or an empty string for an unauthenticated
    /// connection
    pub key: String,
}
```

The client would fill out the connection file like so (note that signature-scheme and key are needed):

```
{
  "handshake_port": 57503,
  "control_port": 0,
  "shell_port": 0,
  "transport": "",
  "signature_scheme": "hmac-sha256",
  "stdin_port": 0,
  "hb_port": 0,
  "ip": "0.0.0.0",
  "iopub_port": 0,
  "key": "a0436f6c-1916-498b-8eb9-e81ab9368e84"
}
```

Ark would see that `handshake_port` is set, and decide to use that, ignoring all other port information.

Internally, ark then finds 5 free ports and immediately binds to them (thus avoiding the race condition). Ark would then utilize the `signature_scheme` and `key` alongside its ports to fill out the "real" connection file, and would not write out `"handshake_port"`:

```
{
  "control_port": 50160,
  "shell_port": 57503,
  "transport": "tcp",
  "signature_scheme": "hmac-sha256",
  "stdin_port": 52597,
  "hb_port": 42540,
  "ip": "127.0.0.1",
  "iopub_port": 40885,
  "key": "a0436f6c-1916-498b-8eb9-e81ab9368e84"
}
```

This connection file would be written to disk somewhere.

Ark would then send the client a new Jupyter Message type I call `ConnectionRequest`:

```rust
pub struct ConnectionRequest {
    /// The path to the connection file created from the handshake
    pub file: String,
}
```

The client would get this `ConnectionRequest` message, read the `file`, and use that information to connect to the ports that ark specified. It would then send back a `ConnectionReply`.

```rust
pub struct ConnectionReply {
}
```

I'm not sure if `ConnectionReply` needs any fields, but when ark receives it it should disconnect from the `handshake_port` socket. If ark has not received a `ConnectionReply` after some amount of time, it should shut itself down.

According to the JEP, ark should also write a `kernel_protocol_version` in its kernelspec that is `>= 5.5` to specify that it supports this mechanism, but I don't think the JEP is formal enough. I've made up some bits in the spec outlined above, so other frontends would have to comply in the exact way I've outlined above for it to work.

## @jmcphers at 2024-10-03T21:58:47Z

@DavisVaughan is there anything in the JEP that suggests we need to write the connection information to disk and send a file path? Seems like it'd be more straightforward to just send a message that includes the connection data itself (plus would work in cross-machine configs)

## @DavisVaughan at 2024-10-03T23:32:02Z

Yes on writing to disk in general

> The kernel should write its connection information in a connection file so that other clients can connect to it.

But its not specific on exactly how the kernel sends that info back to the client

> It then connects to the registration socket and sends the connection information to the registration socket.

So we could totally make `ConnectionRequest` send back that actual info