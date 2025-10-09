# Completions rewrite

> <https://github.com/posit-dev/ark/pull/149>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/1648
Addresses https://github.com/posit-dev/positron/issues/1324
Addresses https://github.com/posit-dev/positron/issues/127 (2 of these are done and working well, i opened individual issues for the others)
Addresses https://github.com/posit-dev/positron/issues/442
Addresses https://github.com/posit-dev/positron/issues/1452
Addresses https://github.com/posit-dev/positron/issues/1255
Addresses https://github.com/posit-dev/positron/issues/1331
Addresses https://github.com/posit-dev/positron/issues/1597
Addresses https://github.com/posit-dev/positron/issues/1735

Includes a change inspired by https://github.com/rstudio/rstudio/issues/13342

This is a large PR that rewrites the completion engine with the main goal of making it modular. Previously, it was too easy for very specific completion types (like file path completions) to get combined with other "general" completions that weren't relevant for that context.

There are now 2 categories of completion _sources_:

- _Unique_ completion sources are special. If we detect we are in a unique source, then those are the _only_ completions that get returned, full stop. If we are in a `lhs$` node, only the names of `lhs` are returned. If we are in a `""`, only file paths are returned. These are example of _unique_ sources.
- _Composite_ completions sources are more general. If we don't find any unique sources, then we "fall back" to a more general set of completions. These include very general things like workspace, document, and search path (like loaded R packages) completions, but also include slightly more specific things like call argument completions and pipe completions (i.e. the object name of the thing on the LHS of the pipe) which can still generally be combined with those other types of completions.

I've tried to reflect this design in the file naming scheme. You'll see folders like:
- `sources/`
    - `unique/`
        - `comment.rs`
        - `file_path.rs`
    - `composite/`
        - `document.rs`
        - `pipe.rs` 

I've fixed a lot of minor bugs along the way, but the underlying implementations of most of these completion sources is mostly still the same as what was there before. The major insight here is restructuring them into unique vs composite.

I've added a set of POC tests to `comment.rs` to test the comment / roxygen2 comment completion source. It required some new testing infrastructure, described below. One of the big benefits of this PR is that we can now more easily test individual pieces of the completion engine. I hope to add more tests like these after we merge this, but this is already too much for one PR so I stopped for now 😬 .

---

Also (unfortunately, sorry!) mixed in is some new infrastructure for testing. I've added an ark level `r_test()` that calls the harp level `r_test()` but _also_ loads the ark level R modules so they can be used in the tests.

I used this in `help.rs` where we were manually trying to do something like this, and it revealed an issue where the help tests would call `.ps.help.showHelpTopic()`, which would eventually try and "show" the help topic, which calls the `browser` hook, and since our modules are now fully loaded that results in `ps_browse_url()` being called, which relies on needing an `RMain`. To avoid this I introduced a new R level global option, `ark.testing`, accessed by `in_ark_tests()`, and used that in `.ps.help.showHelpTopic()` to avoid actually showing the help topic if we are in test mode. This global option is set from ark during module initialization (since it tweaks something managed by the modules folder).

I used the existing `r_poke_option()` to set the global option, but that required some tweaking of that as well, since it uses a ONCE initialized `OPTION_FN` variable, which wasn't actually being initialized right AFAICT. It was only actually being initialized once `r_n_frame()` was run since that managed the ONCE call 😬. I've moved the management of `OPTION_FN` to `utils.rs`, created `init_utils()`, and we now call that from `harp::initialize()` (which previously did nothing), which I've moved into `start_r()` after R is started up.

This one commit captures all of those pieces:
https://github.com/posit-dev/amalthea/pull/149/commits/f88140128d92cc909daa1ce8875aca99d62315cf

## @DavisVaughan at 2023-11-17T22:06:23Z

I've been using this for a few days and it feels pretty solid. There is one bug though that I'll tackle monday. The left open interval `(]` implementation of `find_closest_node_at_point()` means that a point of `(0, 0)` won't ever get matched to any node (because it is the minimum value, and the lhs of the interval is open `(`).

I can reproducibly crash ark by typing `dplyr::across()` in an empty R file and placing my cursor _alllll_ the way at the left in front of the `d`, i.e. at position (0, 0) in the document. Crashes with this (modified error locally to be more useful)

```
[R] 2023-11-17T22:01:18.574359000Z [ark-unknown] ERROR crates/ark/src/lsp/document_context.rs:34: Failed to find closest node to point due to : EmptyOptionError.
[R] 
[R] Point (0, 0)
[R] 
[R] Source
[R] dplyr::across()
[R] 
[R] Backtrace
[R]    0: std::backtrace_rs::backtrace::libunwind::trace
[R]              at /rustc/90c541806f23a127002de5b4038be731ba1458ca/library/std/src/../../backtrace/src/backtrace/libunwind.rs:93:5
[R]    1: std::backtrace_rs::backtrace::trace_unsynchronized
[R]              at /rustc/90c541806f23a127002de5b4038be731ba1458ca/library/std/src/../../backtrace/src/backtrace/mod.rs:66:5
[R]    2: std::backtrace::Backtrace::create
[R]              at /rustc/90c541806f23a127002de5b4038be731ba1458ca/library/std/src/backtrace.rs:332:13
[R]    3: ark::lsp::document_context::DocumentContext::new
[R]              at ./crates/ark/src/lsp/document_context.rs:33:22
[R]    4: <ark::lsp::backend::Backend as tower_lsp::LanguageServer>::hover::{{closure}}
[R]              at ./crates/ark/src/lsp/backend.rs:380:23
[R]    5: <core::pin::Pin<P> as core::future::future::Future>::poll
[R]              at /rustc/90c541806f23a127002de5b4038be731ba1458ca/library/core/src/future/future.rs:125:9
[R]    6: tower_lsp::generated::register_lsp_methods::hover::{{closure}}
[R]              at /Users/davis/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tower-lsp-0.19.0/src/lib.rs:112:1
[R]    7: tower_lsp::jsonrpc::router::Router<S,E>::method::{{closure}}::{{closure}}::{{closure}}
[R]              at /Users/davis/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tower-lsp-0.19.0/src/jsonrpc/router.rs:58:63
[R]    8: <core::pin::Pin<P> as core::future::future::Future>::poll
...
```

I noticed this when working in a Quarto document. For some reason when you open a Quarto document that has a code chunk with something like `options()` in it, then the first time you try and hover over `options()` it will send `Point {0, 0}` to ark and request `hover()` information there. I don't really understand this, it seems like a separate bug.

Hmm, maybe Quarto is doing an initial `render()` of assists, which calls this hover request function, and since that uses the current selection, maybe the initial selection is at point 0, 0? https://github.com/quarto-dev/quarto/blob/1580b78709510f9753b46f3cc3db72525e90faeb/apps/vscode/src/providers/assist/render-assist.ts#L190

## @DavisVaughan at 2023-11-20T19:25:37Z

Fixed up the `Point {0, 0}` crash by changing from our nonstandard `(]` interval to `[]` when determining if a range "contains" a point. This means the LHS edge case of `0, 0` is handled automatically (because it is now contained within the root node if no child contains it, due to the closed bracket `[` on the LHS rather than the open `(`).

I think in general this kind of interval will be easier to understand for maintenance going forward too

## @DavisVaughan at 2023-11-22T21:50:10Z

@lionel- thanks for taking a look and managing the merge! I'll go back and review your feedback on Monday