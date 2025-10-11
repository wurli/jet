# Add `push_rds!()` macro for inspection of R objects in debugging sessions

> <https://github.com/posit-dev/ark/pull/157>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

The Rust debugging experience is currently quite limited. I haven't been able to make library calls into the R API, which prevents inspecting R objects while stepping through code. To work around this limitation, this PR adds a `push_rds!()` macro that makes it easy to collect objects during a debugging session. To use it, add `push_rds!` calls just like you'd use printf statement:

```rust
harp::push_rds!(object);
```

This pushes the object to the RDS file stored in the `RUST_PUSH_RDS_PATH` environment variable. The object is pushed to a data frame with a datetime and context (file, line, and stringified argument):

```
# A tibble: 2 × 3
  date                context                                   x
  <dttm>              <chr>                                     <lis>
1 2023-11-22 14:46:42 crates/ark/src/lsp/completions/sources/c… <fn>
2 2023-11-22 12:00:09 doing this                                <dbl>
```

The PR contains a bunch of supporting changes:

- `r_parse_eval()` now takes the evaluation environment in its option struct.
- `Environment` gains a `bind()` method that wraps `Rf_defineVar()`
- Fixed scoping of `.ps.internal()` to better match `.Internal()`: The function is evaluated in the positron internal namespace but the arguments are evaluated in the calling environment.

I also started the work of removing `unsafe` keywords from the R API:

- `r_parse_eval()` is no longer unsafe
- Added `R_ENVS` global struct containing shortcuts to `global`, `base`, `empty`. These don't require an `unsafe` context.

All this R stuff should only be accessed from the main R thread (`.Call` callbacks or `r_task()` contexts). Making everything `unsafe` is counterproductive though as we become desensitised to the keyword. Instead we should rely on clearer scoping of R accesses in our code (move as much as possible to clearly labellel source files that are only meant to be accessed from the R thread, and then call these via `r_task()`).

