# Add support for alternative `RegistrationFile`

> <https://github.com/posit-dev/ark/pull/576>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Closes #563 
Joint work with @lionel- 

This PR implements the alternative `RegistrationFile` approach outlined in JEP 66 that allows for a "handshake" to occur between the client and server on startup. In particular, it allows the server to be in charge of picking the ports, and immediately binds to them as it picks them, avoiding any race conditions here.

If the `--connection_file` argument provided to ark can parse into this structure:

```rust
pub struct RegistrationFile {
    /// The transport type to use for ZeroMQ; generally "tcp"
    pub transport: String,

    /// The signature scheme to use for messages; generally "hmac-sha256"
    pub signature_scheme: String,

    /// The IP address to bind to
    pub ip: String,

    /// The HMAC-256 signing key, or an empty string for an unauthenticated
    /// connection
    pub key: String,

    /// ZeroMQ port: Registration messages (handshake)
    pub registration_port: u16,
}
```

Then we assume we are going to be using the handshake method of connecting. Otherwise we parse into the typical `ConnectionFile` structure and assume the Client picked the ports.

We expect that the Client _binds_ to a `zmq::REP` socket on `registration_port`. Ark, as the Server, will then _connect_ to this `registration_port` as a `zmq::REQ` socket.

Ark will pick ports, bind to them, and send this message over the registration socket:

```rust
pub struct HandshakeRequest {
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
}
```

Ark will then _immediately_ block, waiting for this `HandshakeReply`:

```rust
pub struct HandshakeReply {
    /// The execution status ("ok" or "error")
    pub status: Status,
}
```

This is just a receipt from the Client that confirms that it received the socket information.

If ark does not receive this reply after a few seconds, it will shut itself down.

Ark disconnects from the registration socket after receiving the `HandshakeReply`, and the kernel proceeds to start up.

Co-authored-by: Lionel Henry <lionel@posit.co>
Co-authored-by: Davis Vaughan <davis@posit.co>

## @lionel- at 2024-10-09T12:59:46Z

TODO for JEP65:

### Subscribe to all subscription messages

```c++
// Set xpub_verbose option to 1 to pass all subscription messages (not only unique ones).
m_publisher.set(zmq::sockopt::xpub_verbose, 1);
```

From the guide (): https://zguide.zeromq.org/docs/chapter5/

> by default, the XPUB socket does not report duplicate subscriptions, which is what you want when you’re naively connecting an XPUB to an XSUB. Our example sneakily gets around this by using random topics so the chance of it not working is one in a million. In a real LVC proxy, you’ll want to use the ZMQ_XPUB_VERBOSE option that we implement in Chapter 6 - The ZeroMQ Community as an exercise.


### IOPub needs to listen on its socket

How do we listen to both XPUB and channel messages?

Have the forwarding thread own the socket? Need to send messages via that thread too...