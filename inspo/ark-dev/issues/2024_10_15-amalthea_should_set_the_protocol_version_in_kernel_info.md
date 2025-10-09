# Amalthea should set the protocol version in Kernel Info

> <https://github.com/posit-dev/ark/issues/578>
>
> * Author: @lionel-
> * State: CLOSED
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABwXx3RQ", name = "area: jupyter kernel", description = "", color = "C2E0C6")

Currently set by Ark and Echo: https://github.com/posit-dev/ark/blob/8b8e869d88c13a90f6b6d0b6790ebcfa11d6078d/crates/ark/src/shell.rs#L175

Amalthea should decide which version is used. For instance we now implement JEP 65 (#577) and the protocol version determines whether a client may depend on this feature.

## @lionel- at 2024-10-10T13:09:52Z

And add a feature flag for JEP 65 in KernelInfo that Kallichore can detect since this feature is not in any official protocol version as of now.

## @DavisVaughan at 2024-10-11T19:52:17Z

Implement PEP 92 for optional `supported_features` send back through `kernel_info_reply`
https://github.com/jupyter/enhancement-proposals/blob/master/92-jupyter-optional-features/jupyter-optional-features.md

I think this is a way smarter approach than relying on the kernel spec version
