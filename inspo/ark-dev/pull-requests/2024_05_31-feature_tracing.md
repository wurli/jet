# Switch to tracing crate for logging and profiling

> <https://github.com/posit-dev/ark/pull/375>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Branched from #371.

The tracing crate has nice amenities for tagging contexts with a name and attributes. These "spans" can be used:

- To provide more context in log messages.
- To profile the time spent in each span throughout the application

This PR switches to this tracing crate in a minimal way. The `log::` crate which we currently use is fully supported and now generates `tracing::` events, so you can still use `log::trace!()` and friends. You can now also use `tracing::trace!()`, which has the benefit of supporting keyed arguments that are nicely formatted as attributes in the log, just as if you had added span attributes to the current context.

A bunch of contexts in Ark now have spans, such as LSP handlers, R tasks, and Jupyter comm messages (these might need a second pass but I added spans in the `handle_rpc()` method). I plan to eventually switch `lsp::log_()` macros to generate tracing events but will do this later on.

There's a new `--profile` argument. This will be set up by the frontend in an ulterior PR. The profiler is based on https://github.com/rust-lang/rust-analyzer/blob/master/crates/rust-analyzer/src/tracing/hprof.rs with some changes to support custom writers.

Our writers use `tracing_appender::non_blocking()` which creates a thread for non-blocking writes.

Here is how the logs look like with span info. Note how the messages indicating how long a thread had to wait before getting its turn on the R event loop are now informative about which thread it was:

<img width="819" alt="Screenshot 2024-05-28 at 11 15 48" src="https://github.com/posit-dev/amalthea/assets/4465050/9ffa9fa2-f431-44c2-8511-ea2dd2d24f3e">

And the profiler:

<img width="710" alt="Screenshot 2024-05-31 at 19 19 28" src="https://github.com/posit-dev/amalthea/assets/4465050/32129eb2-faac-44a1-b8d2-78518524db74">



## @lionel- at 2024-05-31T21:49:49Z

@DavisVaughan oh you know what that's because you're missing the frontend side of things. Sorry I forgot about that. I'll send a PR on Monday: https://github.com/posit-dev/positron/compare/feature/ark-tracing?expand=1

Now if you have `RUST_LOG=trace` you'll get trace level for everything. You need to set `RUST_LOG=warn,ark=trace` to set warning logs for other crates. With this new syntax we can select the verbosity for all other crates, or maybe some particular crate e.g. `RUST_LOG=warn,tokio=trace,ark=trace`.

I added some code to propagate the ark log level to all other internal crates, so `ark=trace` really stands for `ark=trace,amalthea=trace,etc`.

## @lionel- at 2024-06-04T11:05:54Z

With the last commits we now include tracing spans in top-level-exec errors. This actually results in redundant information when the log message (including the one produced in our panic hook if we unwrap the error) is emitted from the same span context than where the error was created. They could potentially carry different information though. We could revert this if this doesn't turn out useful after some usage.
