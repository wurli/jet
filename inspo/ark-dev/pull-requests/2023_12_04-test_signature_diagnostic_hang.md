# Fix diagnostic / completion hang

> <https://github.com/posit-dev/ark/pull/173>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

I can recreate a Windows hang I see by adding in a delay in `signature_help` after it has completed on the R side, but before it has sent off the result to the frontend. This completely freezes up the LSP, strangely in `publish_diagnostics()`, which now never ends.

AFAICT, it really only occurs with `getOption()` "in the wild" on windows, possibly because it has a massive amount of help text that is processed by `signature_help()`, making it slow?? But adding in the sleep like this makes it reproducible with anything.

Note that I am pressing `<tab>` inside the `()` after the function is completed to try and request argument completions. This must occur before the "parameter hints" popup comes up, which is provided by `signature_help()`.

https://github.com/posit-dev/amalthea/assets/19150088/039d6bf0-8446-4b59-8e96-606839f2218c

Notice we never see `diagnostic publish: stop`

```
[R] 2023-12-04T22:44:34.670528000Z [ark-unknown] INFO crates/ark/src/r_task.rs:60: Thread 'ark-lsp' (ThreadId(15)) is requesting a task.
[R] 2023-12-04T22:44:34.670571000Z [ark-unknown] INFO crates/ark/src/interface.rs:1051: Yielding to task - 0 more task(s) remaining
[R] 2023-12-04T22:44:34.816834000Z [ark-unknown] INFO crates/ark/src/r_task.rs:123: Thread 'ark-lsp' (ThreadId(15)) was unblocked after waiting for 146 milliseconds.
[R] 2023-12-04T22:44:34.870171000Z [ark-unknown] INFO crates/ark/src/lsp/backend.rs:416: signature_help: start
[R] 2023-12-04T22:44:34.870237000Z [ark-unknown] INFO crates/ark/src/r_task.rs:60: Thread 'ark-lsp' (ThreadId(15)) is requesting a task.
[R] 2023-12-04T22:44:34.870277000Z [ark-unknown] INFO crates/ark/src/interface.rs:1051: Yielding to task - 0 more task(s) remaining
[R] 2023-12-04T22:44:34.870323000Z [ark-unknown] INFO crates/ark/src/lsp/signature_help.rs:64: Signature help node: ("(")
[R] 2023-12-04T22:44:35.023830000Z [ark-unknown] INFO crates/ark/src/lsp/signature_help.rs:295: SignatureHelp { signatures: [SignatureInformation { label: "getOption(x, default)", documentation: None, parameters: Some([ParameterInformation { label: LabelOffsets([10, 11]), documentation: None }, ParameterInformation { label: LabelOffsets([13, 20]), documentation: None }]), active_parameter: Some(0) }], active_signature: None, active_parameter: Some(0) }
[R] 2023-12-04T22:44:35.024044000Z [ark-unknown] INFO crates/ark/src/r_task.rs:123: Thread 'ark-lsp' (ThreadId(15)) was unblocked after waiting for 153 milliseconds.
[R] 2023-12-04T22:44:35.024074000Z [ark-unknown] INFO crates/ark/src/lsp/backend.rs:418: signature_help: stop
[R] 2023-12-04T22:44:38.029712000Z [ark-unknown] INFO crates/ark/src/r_task.rs:60: Thread 'tokio-runtime-worker' (ThreadId(27)) is requesting a task.
[R] 2023-12-04T22:44:38.029957000Z [ark-unknown] INFO crates/ark/src/interface.rs:1051: Yielding to task - 0 more task(s) remaining
[R] 2023-12-04T22:44:38.051222000Z [ark-unknown] INFO crates/ark/src/r_task.rs:123: Thread 'tokio-runtime-worker' (ThreadId(27)) was unblocked after waiting for 21 milliseconds.
[R] 2023-12-04T22:44:38.052577000Z [ark-unknown] INFO crates/ark/src/lsp/diagnostics.rs:118: diagnostic publish: start
```

## @DavisVaughan at 2023-12-05T15:17:59Z

As noted in https://draft.ryhl.io/blog/shared-mutable-state/, we can get in a nasty state with `async` code and `DashMap`s if we aren't careful.

We absolutely must make sure that any (mutable) references to entries in the `DashMap` are dropped before calling an `await`. Otherwise we can get into a deadlock scenario (described in a comment in the commit)
