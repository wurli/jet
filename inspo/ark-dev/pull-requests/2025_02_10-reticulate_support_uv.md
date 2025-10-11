# Support for newer reticulate

> <https://github.com/posit-dev/ark/pull/703>
>
> * Author: @dfalbel
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/6252

Starting with reticulate v1.41 (currently dev version), `py_discover_config()` might return `NULL`, but `py_config()` will correctly resolve and create a proper temprary environment for the session. Thus we allow skipping the `py_discover_config()` step.

cc @t-kalinowski

