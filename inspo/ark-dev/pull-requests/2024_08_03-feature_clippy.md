# Initial round of clippy warnings

> <https://github.com/posit-dev/ark/pull/461>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Closes https://github.com/posit-dev/ark/pull/436

@yutannihilation I've reproduced your workflow over here, just to get comfortable with it and to have it rebased on current main, but thanks a lot for getting it started. This PR fixes the same lints you started with, and I'll add more next week-ish.

As @yutannihilation said over in that other PR, the key to working these out one at a time is

```
cargo clippy --fix -- -A clippy::all -W clippy::vec_init_then_push
```

- `--fix` to auto fix issues
- `-A clippy::all` to "allow all clippy issues", basically silencing everything else
- `-W clippy::vec_init_then_push` to then force 1 particular issue as a warning

If you just want to see the issues, remove the `--fix` and it won't auto fix them.

`--fix` doesn't auto fix every issue. For those it can't auto fix, it will show a warning in the terminal.

It is typically worth going through the changes manually, because you can occasionally (like 5% of the time) make another simplification in the code based on the fix.

