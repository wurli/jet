# Switch to nextest for testing

> <https://github.com/posit-dev/ark/pull/753>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

This PR switches us to using nextest for our testing https://nexte.st/. It also adds a `justfile` so we can use `just test` and `just test-insta` to easily invoke nextest. We have enjoyed this setup over on Air so far. 

You will need to install nextest and just binaries locally, see below.

Importantly this doesn't do much besides switching us over to nextest (I've done some prep work leading up to this in other PRs).

## Why are we doing this?

The selling point of nextest for us is _one process per test_ https://nexte.st/docs/design/why-process-per-test/. Typical `cargo test` is one process per binary - but tests within the binary run in parallel on different threads within that process. This has caused us great headaches, as we are trying to share an R session (and other global state) between tests running in parallel. We've recently dealt with one pain point related to this but there have been many others https://github.com/posit-dev/ark/pull/740.

After this PR, we can incrementally simplify our testing structure (removing locks that are no longer required, simplifying assumptions, etc) to fully take advantage of nextest, but it should "just work" even with our current setup. Each test is now getting its own R process.

## Installing nextest

There are pre built binaries of nextest that you will need to install. If you are on macOS:

```bash
curl -LsSf https://get.nexte.st/latest/mac | tar zxf - -C ${CARGO_HOME:-~/.cargo}/bin
```

## Installing just

Just is a command runner https://github.com/casey/just. If you are on macOS:

```bash
brew install just
```

## Running tests

```bash
# To run tests, invokes `cargo nextest run`
just test

# To update insta snapshots
just test-insta
```

I highly recommend setting up this user level keybinding for `Cmd+Shift+T`:

```json
[
    {
        "key": "shift+cmd+t",
        "command": "workbench.action.tasks.test"
    },
    {
        "key": "shift+cmd+t",
        "command": "-workbench.action.reopenClosedEditor"
    }
]
```

## What do we lose?

Once we start simplifying our test structure, you absolute cannot expect `cargo test` to "just work" anymore. We will have invariants that expect one process per test and it just doesn't do that. Use `just test` instead.

With rust-analyzer, you can still use the green play button to run an _individual_ test but you can no longer use the green play button to run a _mod of tests_. You won't get a slap on the wrist or anything, it will just crash and burn. I really wish rust-analyzer had some nextest integration for this.

