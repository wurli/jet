# Teach extractor completion source about `identifier`s

> <https://github.com/posit-dev/ark/pull/326>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/2300

Our `$` and `@` completions source code was able to generate completions for `foo$` but if you start typing `foo$lin` then it saw that `lin` identifier and failed to "look up" one level to see that you are also still in a `$` node. It was declining to handle completions for that case, meaning that the default composite completions were running, giving you completions for general functions like `lines()` and whatnot.

I've made it much more robust, and added quite a few additional tests for things like:
- Being on the LHS of the `$`, like `foo@$lin`
- Having a nonexistent object on the LHS, like `nonexistent$@`
- Having a "too complex" LHS, like `list(a = 1)$@`
- Being in the middle of an identifier on the rhs, like `x$a@bc`

The "nonexistent" object and "too complex" cases in particular would previously cause an error to get logged to the LSP output channel of something like this, which was harmless but noisy and incorrect for what we were trying to do here.

```
Failed to provide completions: Error evaluating nonexistent: Error: object 'nonexistent' not found


Stack backtrace:
   0: std::backtrace_rs::backtrace::libunwind::trace
```

```
Failed to provide completions: Evaluation of function calls not supported in this context: list(a = 1)

Stack backtrace:
   0: std::backtrace_rs::backtrace::libunwind::trace
             at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/std/src/../../backtrace/src/backtrace/libunwind.rs:104:5
   1: std::backtrace_rs::backtrace::trace_unsynchronized
```

