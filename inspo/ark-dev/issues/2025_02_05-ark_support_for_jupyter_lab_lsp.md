# Ark: Support for JupyterLab-LSP

> <https://github.com/posit-dev/ark/issues/699>
>
> * Author: @lionel-
> * State: OPEN
> * Labels:

I would have loved to show Ark completions running in Jupyter lab in our Posit::conf talk, so I looked into how this could be done in case there's an easy way to set it up. Unfortunately [jupyterlab-lsp](https://github.com/jupyter-lsp/jupyterlab-lsp) currently requires a static spec file describing how to launch the server and an stdio connection instead of TCP (see https://jupyterlab-lsp.readthedocs.io/en/latest/Configuring.html). These requirements make it too tricky to implement in short order.

For the longer term, here are the two ways I can think of for implementing this:

1. Split the LSP from the kernel (https://github.com/posit-dev/positron/issues/3180) and use a regular jupyter-lsp spec file to launch an LSP instance. This has the downside that the LSP will not be connected to the kernel and will not know about dynamic state such as variables in the global environment.

2. Work with the jupyterlab people to formalise a Jupyter request message for which the kernel responds with a connection spec containing the arguments that allow a second instance of Ark to connect to the kernel.

The second option could work whether we have fully split the LSP from the kernel or not.

- If we have, this LSP instance will analyse the project independently. To get updated about dynamic state after top-level commands it connects to the kernel via 0MQ and subscribes to IOPub.

- If we haven't split the LSP from the kernel, we just use the separate Ark instance as a relay to the instance in the kernel. It would connect to the kernel LSP via TCP.

This last scenario is the one that requires the least amount of changes to Ark, but we do need jupyterlab requesting the spec from our kernel instead of using a static one.

## @lionel- at 2024-09-23T09:16:14Z

We should also implement support for Jupyter's `complete_request`: https://jupyter-client.readthedocs.io/en/stable/messaging.html#completion

The Ark handler currently returns the empty set: https://github.com/posit-dev/ark/blob/0e26941d0f8e7dffc8ce57eb5dbeedc13cf7f2ef/crates/ark/src/shell.rs#L181

## @lionel- at 2024-09-23T11:45:22Z

Upstream issue: https://github.com/jupyter-lsp/jupyterlab-lsp/issues/1099
