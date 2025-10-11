# Log backtrace on sigsegv and sigbus signals

> <https://github.com/posit-dev/ark/pull/22>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Requires https://github.com/rstudio/positron/pull/708 otherwise the backtrace doesn't show up.

<img width="848" alt="Screenshot 2023-06-08 at 18 14 29" src="https://github.com/posit-dev/amalthea/assets/4465050/769d2a93-c292-4c7a-8dee-fe5a1a6c9252">


## @lionel- at 2023-06-08T16:26:47Z

I guess I should notify the next handler. Rust handles segfaults to detect and report stack overflows: https://github.com/rust-lang/rust/pull/31333

## @lionel- at 2023-06-09T07:35:32Z

> I guess I should notify the next handler. Rust handles segfaults to detect and report stack overflows

To do this robustly we need a separate implementation for Windows and Unix so this doesn't seem worth it. I added a note about that.

I also added a note about R's segfault handler. It doesn't override our handler because we set `R_SignalHandlers` to 0 before startup.
