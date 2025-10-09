# Only re-check if we won't overflow

> <https://github.com/posit-dev/ark/pull/198>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

I managed to reproduce a panic I was seeing by putting this in a package:

```r
(foo) <- function() {

}
```

Then placing the cursor before the first `(`, and right clicking and hitting "find references"

```
thread 'ark-lsp' panicked at 'attempt to subtract with overflow', crates/ark/src/lsp/references.rs:115:51
stack backtrace:
   0: rust_begin_unwind
             at /rustc/90c541806f23a127002de5b4038be731ba1458ca/library/std/src/panicking.rs:578:5
   1: core::panicking::panic_fmt
             at /rustc/90c541806f23a127002de5b4038be731ba1458ca/library/core/src/panicking.rs:67:14
   2: core::panicking::panic
             at /rustc/90c541806f23a127002de5b4038be731ba1458ca/library/core/src/panicking.rs:117:5
   3: ark::lsp::references::<impl ark::lsp::backend::Backend>::build_context::{{closure}}
             at /Users/davis/files/programming/positron/amalthea/crates/ark/src/lsp/references.rs:115:51
   4: ark::lsp::backend::Backend::with_document
             at /Users/davis/files/programming/positron/amalthea/crates/ark/src/lsp/backend.rs:101:16
   5: ark::lsp::references::<impl ark::lsp::backend::Backend>::build_context
             at /Users/davis/files/programming/positron/amalthea/crates/ark/src/lsp/references.rs:99:23
   6: ark::lsp::references::<impl ark::lsp::backend::Backend>::find_references
             at /Users/davis/files/programming/positron/amalthea/crates/ark/src/lsp/references.rs:221:31
   7: <ark::lsp::backend::Backend as tower_lsp::LanguageServer>::references::{{closure}}
             at /Users/davis/files/programming/positron/amalthea/crates/ark/src/l
```

Note that the `column` position is `0` there.

We do this two stage check to "figure out where we are" when looking for object references and in the 2nd stage we have to shift the column position back by `1` column. But if we are already at column `0`, this underflows and gives us a panic.

