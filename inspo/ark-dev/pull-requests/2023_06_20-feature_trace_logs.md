# Set amalthea workspace log level to `"trace"`

> <https://github.com/posit-dev/ark/pull/47>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

As follow up to https://github.com/rstudio/positron/pull/752

Most people don't need trace logs, so https://github.com/rstudio/positron/pull/752 defaults to `"warn"` for the R logs. But if you are working on amalthea directly, you probably do want `"trace"` logs, so this updates the Workspace settings so you get that by default, and can override on an ad hoc basis as needed

