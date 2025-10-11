# Generate virtual documents for namespace functions without source refs

> <https://github.com/posit-dev/ark/pull/251>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/2285
Addresses part of https://github.com/posit-dev/positron/issues/1729
Addresses part of https://github.com/posit-dev/positron/issues/29

Closes https://github.com/posit-dev/positron/issues/2284
Requires https://github.com/posit-dev/positron/pull/2320

When source references for a namespace are missing, we now deparse the functions into a virtual document. On the frontend side this document is accessible via the `ark:` URI scheme. Since we don't know the original file organisation of the package, the whole namespace is stored in a single source file, e.g. `ark:namespace:base.R` for base.

This is all that is needed to support stepping with the debugger (https://github.com/posit-dev/positron/issues/1729). Supporting jump-to-definition and find-usages will require more work.

Supportive new features implemented here:

- Idle tasks (https://github.com/posit-dev/positron/issues/2284)
- General `onLoad` event. R only supports `onLoad` hooks for specific namespaces known in advance. We extend the hook system to generate an event in Ark for the loading of any namespace.
- The environment iterator gains an `assign()` method that is used to update the namespaces in place
- The environment iterator now supports a new variant implementation for the base namespace. It requires a specific path because the namespace is stored in the symbol table instead of a linked list or hash table.

The process goes as follows:

- Idle tasks are registered to populate a virtual source for the namespace for:
  - All already loaded namespaces (base, utils, ...)
  - All newly loaded namespaces via the new event

- In a namespace, look for all functions missing source refs. Deparse the functions to text and reparse them to an AST with source references pointing to the virtual document. Supported by the `srcfile` argument fo `parse()` and line directives `#line n` added to the deparsed source.

- If the AST does not correspond exactly, discard it and ignore this function. This happens when the function has been generated programmatically or when there is a bug in the deparser, e.g. `foo$'bar'` is deparsed as `foo$bar` and so the roundtrip doesn't correspond exactly.

  In these cases debugger stepping will be supported by a fallback path (https://github.com/posit-dev/amalthea/pull/249)

- If it corresponds, update the function in place. We use a bit of a hacky trick to reuse the bytecode of the original function because recompiling is much too slow.


## @DavisVaughan at 2024-02-27T15:20:43Z

First anecdotal note - starting up R is now _much_ slower. i.e. opening dplyr now leaves me in the `R starting` state for roughly 6-7 seconds before switching to `started`. In the output I see that it seems to be due to processing the virtual namespaces because this pops up right after R switches to `started`

```
[R] 2024-02-27T15:17:18.070905000Z [ark-unknown] TRACE crates/ark/src/srcref.rs:100: Populated virtual namespace for cli: 3 lines, 0 ok, 1 bad, 683 skipped
[R] 2024-02-27T15:17:18.252994000Z [ark-unknown] TRACE crates/ark/src/srcref.rs:100: Populated virtual namespace for rlang: 12367 lines, 1219 ok, 2 bad, 309 skipped
[R] 2024-02-27T15:17:18.275440000Z [ark-unknown] TRACE crates/ark/src/srcref.rs:100: Populated virtual namespace for compiler: 2206 lines, 139 ok, 0 bad, 150 skipped
[R] 2024-02-27T15:17:18.341081000Z [ark-unknown] TRACE crates/ark/src/srcref.rs:100: Populated virtual namespace for graphics: 5108 lines, 123 ok, 3 bad, 45 skipped
[R] 2024-02-27T15:17:18.976654000Z [ark-unknown] TRACE crates/ark/src/srcref.rs:100: Populated virtual namespace for tools: 34109 lines, 701 ok, 1 bad, 76 skipped
[R] 2024-02-27T15:17:19.075554000Z [ark-unknown] TRACE crates/ark/src/srcref.rs:100: Populated virtual namespace for utils: 15913 lines, 496 ok, 1 bad, 72 skipped
[R] 2024-02-27T15:17:19.224648000Z [ark-unknown] TRACE crates/ark/src/srcref.rs:100: Populated virtual namespace for grDevices: 3942 lines, 213 ok, 0 bad, 75 skipped
[R] 2024-02-27T15:17:19.575379000Z [ark-unknown] TRACE crates/ark/src/srcref.rs:100: Populated virtual namespace for stats: 24387 lines, 901 ok, 14 bad, 230 skipped
[R] 2024-02-27T15:17:19.575464000Z [ark-unknown] TRACE crates/ark/src/srcref.rs:100: Populated virtual namespace for datasets: 3 lines, 0 ok, 0 bad, 3 skipped
[R] 2024-02-27T15:17:19.858894000Z [ark-unknown] TRACE crates/ark/src/srcref.rs:100: Populated virtual namespace for methods: 12046 lines, 512 ok, 0 bad, 249 skipped
[R] 2024-02-27T15:17:20.082611000Z [ark-unknown] TRACE crates/ark/src/srcref.rs:100: Populated virtual namespace for base: 17372 lines, 1126 ok, 12 bad, 250 skipped
```

I am somewhat certain that our r-idle-async tasks end up just running _immediately_ and block because of this recursive case check, since `resource_loaded_namespaces()` is called inside `start_r()`, which is on the main r thread
https://github.com/posit-dev/amalthea/blob/b67690c117780016ec614202b0f76bba84e20e5c/crates/ark/src/r_task.rs#L169

https://github.com/posit-dev/amalthea/assets/19150088/4ed9ed81-09e0-4c1c-93b4-560ba1b03c43




## @DavisVaughan at 2024-02-27T15:34:20Z

Running `load_all()` in dplyr hits me with

```r
> devtools::load_all()
ℹ Loading dplyr
Warning message:
Problem while running user `onLoad` hook for package dplyr.
ℹ The hook inherits from `package:base`.
Caused by error in `fun()`:
! argument "path" is missing, with no default
```

Ah this seems to be because pkgload doesn't supply the 2nd `path` argument, unlike base R
https://github.com/r-lib/pkgload/blob/7556a3f0a74e37afd5286b126b6b2321e563e761/R/run-loadhooks.R#L59

https://github.com/wch/r-source/blob/96e3692a7587782e2d6acb138c84b588607a8024/src/library/base/R/namespace.R#L239-L242

## @DavisVaughan at 2024-02-27T16:22:28Z

Unexpected longjmp error I've never seen before. I was typing this

```
library(dplyr)

debugonce(vctrs::vec_slice)

arrange(mtcars, ) # typing this line
```

It seems to truly be related to the `debugonce(vctrs::vec_slice)`

Full traceback below, but here are some related bits

```
[R] Likely caused by: Error: attempt to apply non-function
```

```
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1140:12
[R]  251: libr::r::Rf_eval
[R]              at /Users/davis/files/programming/positron/amalthea/crates/libr/src/functions.rs:31:21
[R]  252: harp::exec::RFunction::call_in
[R]              at /Users/davis/files/programming/positron/amalthea/crates/harp/src/exec.rs:165:38
[R]  253: harp::exec::RFunction::call
[R]              at /Users/davis/files/programming/positron/amalthea/crates/harp/src/exec.rs:108:9
[R]  254: ark::lsp::help::RHtmlHelp::new
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/lsp/help.rs:40:24
[R]  255: ark::lsp::completions::resolve::resolve_function_completion_item
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/lsp/completions/resolve.rs:74:24
[R]  256: ark::lsp::completions::resolve::resolve_completion
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/lsp/completions/resolve.rs:33:13
[R]  257: <ark::lsp::backend::Backend as tower_lsp::LanguageServer>::completion_resolve::{{closure}}::{{closure}}
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/lsp/backend.rs:367:41
[R]  258: ark::r_task::r_task::{{closure}}
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/r_task.rs:70:44
```

<details>

```
[R] 2024-02-27T16:18:57.177291000Z [ark-unknown] ERROR crates/ark/src/main.rs:425: Panic! In file 'crates/ark/src/interface.rs' at line 1281: Unexpected longjump while reading console: `R_topLevelExec()` error: Unexpected longjump.
[R] Likely caused by: Error: attempt to apply non-function
[R]    0: std::backtrace_rs::backtrace::libunwind::trace
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/std/src/../../backtrace/src/backtrace/libunwind.rs:104:5
[R]    1: std::backtrace_rs::backtrace::trace_unsynchronized
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/std/src/../../backtrace/src/backtrace/mod.rs:66:5
[R]    2: std::backtrace::Backtrace::create
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/std/src/backtrace.rs:331:13
[R]    3: harp::exec::r_top_level_exec
[R]              at /Users/davis/files/programming/positron/amalthea/crates/harp/src/exec.rs:430:24
[R]    4: harp::exec::r_sandbox
[R]              at /Users/davis/files/programming/positron/amalthea/crates/harp/src/exec.rs:565:5
[R]    5: r_read_console
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/interface.rs:1278:18
[R]    6: Rf_ReplIteration
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/main.c:210:10
[R]    7: R_ReplConsole
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/main.c:314:11
[R]    8: do_browser
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/main.c:1405:2
[R]    9: R_execClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2166:2
[R]   10: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]   11: R_forceAndCall
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2254:8
[R]   12: do_lapply
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/apply.c:75:8
[R]   13: do_internal
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/names.c:1404:11
[R]   14: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7466:15
[R]   15: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   16: R_execClosure
[R]   17: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]   18: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]   19: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   20: R_execClosure
[R]   21: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]   22: dispatchMethod
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/objects.c:399:16
[R]   23: Rf_usemethod
[R]   24: tryDispatch
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5694:7
[R]   25: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7592:25
[R]   26: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   27: R_execClosure
[R]   28: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]   29: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]   30: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   31: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]   32: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]   33: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]   34: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]   35: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   36: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]   37: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]   38: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]   39: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]   40: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   41: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]   42: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]   43: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]   44: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]   45: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   46: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]   47: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]   48: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]   49: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]   50: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   51: R_execClosure
[R]   52: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]   53: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]   54: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   55: R_execClosure
[R]   56: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]   57: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]   58: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   59: R_execClosure
[R]   60: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]   61: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]   62: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   63: R_execClosure
[R]   64: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]   65: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]   66: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   67: R_execClosure
[R]   68: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]   69: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1140:12
[R]   70: do_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:3625:13
[R]   71: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7446:14
[R]   72: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   73: R_execClosure
[R]   74: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]   75: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]   76: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   77: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]   78: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1040:9
[R]   79: do_withVisible
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:3677:9
[R]   80: do_internal
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/names.c:1404:11
[R]   81: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7466:15
[R]   82: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   83: R_execClosure
[R]   84: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]   85: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]   86: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   87: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]   88: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]   89: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]   90: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]   91: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   92: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]   93: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]   94: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]   95: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]   96: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]   97: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]   98: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]   99: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]  100: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]  101: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  102: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]  103: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]  104: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]  105: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]  106: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  107: R_execClosure
[R]  108: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  109: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  110: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  111: R_execClosure
[R]  112: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  113: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  114: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  115: R_execClosure
[R]  116: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  117: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  118: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  119: R_execClosure
[R]  120: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  121: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  122: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  123: R_execClosure
[R]  124: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  125: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  126: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  127: R_execClosure
[R]  128: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  129: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  130: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  131: R_execClosure
[R]  132: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  133: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  134: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  135: R_execClosure
[R]  136: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  137: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  138: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  139: R_execClosure
[R]  140: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  141: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  142: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  143: R_execClosure
[R]  144: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  145: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  146: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  147: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]  148: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]  149: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]  150: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]  151: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  152: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]  153: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]  154: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]  155: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]  156: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  157: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]  158: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]  159: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]  160: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]  161: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  162: R_execClosure
[R]  163: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  164: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  165: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  166: R_execClosure
[R]  167: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  168: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  169: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  170: R_execClosure
[R]  171: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  172: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  173: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  174: R_execClosure
[R]  175: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  176: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  177: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  178: R_execClosure
[R]  179: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  180: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  181: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  182: R_execClosure
[R]  183: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  184: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  185: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  186: R_execClosure
[R]  187: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  188: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1140:12
[R]  189: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]  190: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]  191: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]  192: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]  193: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  194: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]  195: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]  196: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]  197: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]  198: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  199: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]  200: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]  201: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]  202: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]  203: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  204: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]  205: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]  206: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]  207: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]  208: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  209: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]  210: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]  211: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]  212: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]  213: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  214: R_execClosure
[R]  215: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  216: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  217: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  218: R_execClosure
[R]  219: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  220: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  221: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  222: R_execClosure
[R]  223: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  224: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  225: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  226: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]  227: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]  228: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]  229: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]  230: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  231: forcePromise
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:833:8
[R]  232: FORCE_PROMISE
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5467:15
[R]  233: getvar
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:5508:14
[R]  234: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7198:20
[R]  235: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  236: R_execClosure
[R]  237: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  238: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  239: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  240: R_execClosure
[R]  241: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  242: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  243: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  244: R_execClosure
[R]  245: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  246: bcEval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:7414:12
[R]  247: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1013:8
[R]  248: R_execClosure
[R]  249: Rf_applyClosure
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:2113:16
[R]  250: Rf_eval
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/eval.c:1140:12
[R]  251: libr::r::Rf_eval
[R]              at /Users/davis/files/programming/positron/amalthea/crates/libr/src/functions.rs:31:21
[R]  252: harp::exec::RFunction::call_in
[R]              at /Users/davis/files/programming/positron/amalthea/crates/harp/src/exec.rs:165:38
[R]  253: harp::exec::RFunction::call
[R]              at /Users/davis/files/programming/positron/amalthea/crates/harp/src/exec.rs:108:9
[R]  254: ark::lsp::help::RHtmlHelp::new
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/lsp/help.rs:40:24
[R]  255: ark::lsp::completions::resolve::resolve_function_completion_item
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/lsp/completions/resolve.rs:74:24
[R]  256: ark::lsp::completions::resolve::resolve_completion
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/lsp/completions/resolve.rs:33:13
[R]  257: <ark::lsp::backend::Backend as tower_lsp::LanguageServer>::completion_resolve::{{closure}}::{{closure}}
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/lsp/backend.rs:367:41
[R]  258: ark::r_task::r_task::{{closure}}
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/r_task.rs:70:44
[R]  259: core::ops::function::FnOnce::call_once{{vtable.shim}}
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/core/src/ops/function.rs:250:5
[R]  260: <alloc::boxed::Box<F,A> as core::ops::function::FnOnce<Args>>::call_once
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/alloc/src/boxed.rs:2007:9
[R]  261: harp::exec::r_top_level_exec::c_fn
[R]              at /Users/davis/files/programming/positron/amalthea/crates/harp/src/exec.rs:417:28
[R]  262: R_ToplevelExec
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/context.c:799:2
[R]  263: libr::r::R_ToplevelExec
[R]              at /Users/davis/files/programming/positron/amalthea/crates/libr/src/functions.rs:31:21
[R]  264: harp::exec::r_top_level_exec
[R]              at /Users/davis/files/programming/positron/amalthea/crates/harp/src/exec.rs:420:28
[R]  265: harp::exec::r_sandbox
[R]              at /Users/davis/files/programming/positron/amalthea/crates/harp/src/exec.rs:565:5
[R]  266: ark::r_task::RTaskMain::fulfill
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/r_task.rs:220:22
[R]  267: ark::interface::RMain::yield_to_tasks
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/interface.rs:1042:21
[R]  268: ark::interface::RMain::read_console
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/interface.rs:639:13
[R]  269: ark::interface::r_read_console::{{closure}}
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/interface.rs:1278:31
[R]  270: harp::exec::r_top_level_exec::c_fn
[R]              at /Users/davis/files/programming/positron/amalthea/crates/harp/src/exec.rs:417:28
[R]  271: R_ToplevelExec
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/context.c:799:2
[R]  272: libr::r::R_ToplevelExec
[R]              at /Users/davis/files/programming/positron/amalthea/crates/libr/src/functions.rs:31:21
[R]  273: harp::exec::r_top_level_exec
[R]              at /Users/davis/files/programming/positron/amalthea/crates/harp/src/exec.rs:420:28
[R]  274: harp::exec::r_sandbox
[R]              at /Users/davis/files/programming/positron/amalthea/crates/harp/src/exec.rs:565:5
[R]  275: r_read_console
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/interface.rs:1278:18
[R]  276: Rf_ReplIteration
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/main.c:210:10
[R]  277: R_ReplConsole
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/main.c:314:11
[R]  278: run_Rmainloop
[R]              at /Volumes/Builds/R4/R-4.3.1/src/main/main.c:1200:5
[R]  279: libr::r::run_Rmainloop
[R]              at /Users/davis/files/programming/positron/amalthea/crates/libr/src/functions.rs:31:21
[R]  280: ark::sys::unix::interface::run_r
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/sys/unix/interface.rs:79:9
[R]  281: ark::interface::start_r
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/interface.rs:224:5
[R]  282: ark::start_kernel
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/main.rs:131:5
[R]  283: ark::parse_file
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/main.rs:208:13
[R]  284: ark::main
[R]              at /Users/davis/files/programming/positron/amalthea/crates/ark/src/main.rs:440:9
[R]  285: core::ops::function::FnOnce::call_once
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/core/src/ops/function.rs:250:5
[R]  286: std::sys_common::backtrace::__rust_begin_short_backtrace
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/std/src/sys_common/backtrace.rs:154:18
[R]  287: std::rt::lang_start::{{closure}}
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/std/src/rt.rs:167:18
[R]  288: core::ops::function::impls::<impl core::ops::function::FnOnce<A> for &F>::call_once
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/core/src/ops/function.rs:284:13
[R]  289: std::panicking::try::do_call
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/std/src/panicking.rs:552:40
[R]  290: std::panicking::try
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/std/src/panicking.rs:516:19
[R]  291: std::panic::catch_unwind
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/std/src/panic.rs:142:14
[R]  292: std::rt::lang_start_internal::{{closure}}
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/std/src/rt.rs:148:48
[R]  293: std::panicking::try::do_call
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/std/src/panicking.rs:552:40
[R]  294: std::panicking::try
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/std/src/panicking.rs:516:19
[R]  295: std::panic::catch_unwind
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/std/src/panic.rs:142:14
[R]  296: std::rt::lang_start_internal
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/std/src/rt.rs:148:20
[R]  297: std::rt::lang_start
[R]              at /rustc/82e1608dfa6e0b5569232559e3d385fea5a93112/library/std/src/rt.rs:166:17
[R]  298: _main
[R]
[R]
```

</details>

It seems to be hitting reproducibly when just typing in `arran`...

---

Update, ok for some unknown reason we actually do hit `vctrs::vec_slice()` when `tools:::Rd2HTML()` runs??? So the debugonce event is trying to fire as we are trying to load completions, but something isn't working right anymore (it works better with `main` so you can at least see what is going on)

## @lionel- at 2024-02-28T10:36:31Z

> First anecdotal note - starting up R is now much slower. i.e. opening dplyr now leaves me in the R starting state for roughly 6-7 seconds before switching to started.

Interesting, I noticed a slowdown too but much smaller.

> I am somewhat certain that our r-idle-async tasks end up just running immediately and block because of this recursive case check, since resource_loaded_namespaces() is called inside start_r(), which is on the main r thread

oh that makes sense, I'll fix that by never running immediately in the case of idle tasks

## @lionel- at 2024-06-06T13:54:07Z

@DavisVaughan Ready for review!
