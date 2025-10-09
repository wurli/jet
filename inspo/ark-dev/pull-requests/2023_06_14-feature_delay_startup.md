# Add `--delay-startup` argument to Ark

> <https://github.com/posit-dev/ark/pull/35>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Progress towards rstudio/positron#740

Adds a new undocumented argument `--startup-delay`. It takes the path to a notification file to which the frontend writes when the debugger has been attached. Ark blocks until a change to the file is detected. After that, startup continues as normal.

## @lionel- at 2023-06-15T10:44:41Z

> This LGTM, but I wouldn't have expected an argument called --delay-startup to take a file argument.

Good point. I've renamed the argument to `--startup-notifier-file`.

> Maybe if the argument is numeric we could sleep that number of seconds instead, to support more traditional debugger scenarios?

Good idea. I think I'd implement this with a new `--startup-delay` argument to keep things simple. Otherwise a typo in the supplied argument that causes a parse failure will produce surprising behaviour instead of an error. Now done in last commit.