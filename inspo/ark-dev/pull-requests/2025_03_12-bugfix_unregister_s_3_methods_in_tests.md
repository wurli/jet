# Better hygiene around S3 methods in tests

> <https://github.com/posit-dev/ark/pull/740>
> 
> * Author: @jennybc
> * State: MERGED
> * Labels: 

While working on something else, I noticed a flaky test:

```
failures:

---- variables::variable::tests::test_matrix_display stdout ----

thread 'variables::variable::tests::test_matrix_display' panicked at crates/ark/src/variables/variable.rs:2133:13:
assertion `left == right` failed
  left: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
 right: "[1 row x 1 column] <foo>"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    variables::variable::tests::test_matrix_display
```

Discussed with @DavisVaughan who offered some very helpful analysis: this comes from two different tests being run in the same R session ?in parallel or in some random order?. Both have assertions that tickle the `ark_positron_variable_display_value` method for a toy `"foo"` class and they can crosstalk with each other.

Again at @DavisVaughan's suggestion, I am introducing and using a method to unregister an S3 method in ark's method table.

I have very low-tech empirical proof that it's working. Before and after this PR, I ran `cargo test --package ark --lib variables::variable::tests` 10 times. Before, I saw 2 failures (the exact one shown above) and 8 passes. After, I see 10 passes.

