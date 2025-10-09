# Jupyter artifacts when Ark is used outside of Positron

> <https://github.com/posit-dev/ark/issues/652>
> 
> * Author: @grantmcdermott
> * State: CLOSED
> * Labels: 

Hi folks. Very excited by Ark. Thanks for all the work on it.

Myself and a few colleagues have been trying Ark outside of Positron, e.g. in Zed or regular Jupyter Notebook session. Some&mdash;but weirdly not all&dash;of us are seeing undesirable Jupyter (JSON?) logging artifacts alongside the regular cell output. For example, running 

```r
# %%
plot(1:10)
```

in Zed produces the plot correctly...but preempted with the following cruft.

```
  2024-12-09T19:55:07.910014Z  INFO  Received subscribe message on IOPub with subscription ''.
    at crates/amalthea/src/socket/iopub.rs:260
  2024-12-09T19:55:07.912842Z  INFO  Received shell request: JupyterMessage { zmq_identities: [[0, 128, 0, 65, 167]], header: Ju
pyterHeader { msg_id: "f8cc6be9-e09c-4c9f-b452-4620993a1389", session: "bdaca3cc-a480-47f1-9a7e-d0bf701334da", username: "runtim
elib", date: "2024-12-09T19:55:06.454278Z", msg_type: "execute_request", version: "5.3" }, parent_header: None, content: Execute
Request { code: "# %%\nplot(1)", silent: false, store_history: true, user_expressions: Object {}, allow_stdin: false, stop_on_er
ror: true } }
    at crates/amalthea/src/socket/shell.rs:185
```

Similar result in a native Jupyter session.

Is there a way to silence or remove these artifacts? Apologies if I missed a duplicate issue.

I'm running the latest Ark release `0.1.158` on MacOS (Sequoia).

## @lionel- at 2024-12-12T09:24:18Z

Have you installed the kernel spec with `ark --install` and a recent version of Ark?

This should set `RUST_LOG` to `error`:

```json
"env": {
     "RUST_LOG": "error"
}
```

## @grantmcdermott at 2024-12-12T18:20:21Z

Thanks @lionel-.

It's bit odd since I had already installed the kernel spec... but reinstalling seems to have fixed the issue. I did, however, encounter the spurious `--file-connection` error mentioned in https://github.com/posit-dev/ark/issues/648

Appreciate the help.
