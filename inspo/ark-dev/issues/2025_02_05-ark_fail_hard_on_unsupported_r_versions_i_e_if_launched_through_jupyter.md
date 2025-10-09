# Ark: Fail hard on unsupported R versions (i.e. if launched through Jupyter)

> <https://github.com/posit-dev/ark/issues/696>
> 
> * Author: @DavisVaughan
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABSPDwBw", name = "bug", description = "Something isn't working", color = "E99695")

We need to make sure that ark itself is explicit about the fact that we only support R >=4.2.0, so if we detect that we are being launched with an older version of R, we should have some way of failing early with some informative error in the log.

Positron itself is good about not allowing users to start R < 4.2.0, but that won't be the case in general if ark is used as a kernel.

## @lionel- at 2024-06-10T11:14:37Z

Should we also fail when using outdated versions of rlang, cli, etc?