# Fix segfault in variables tests

> <https://github.com/posit-dev/ark/pull/641>
>
> * Author: @dfalbel
> * State: MERGED
> * Labels:

With current main, running:

```
cargo test -p ark --lib variables::variable::tests::test_truncation
```

Will eventually fail with:

```
Error: VECTOR_ELT() can only be applied to a 'list', not a 'symbol'
Fatal error: unable to initialize the JIT
```

While I'm not entirely sure what's happenning, I believe this is a protection issue, and this PR seems to fix the problem.

