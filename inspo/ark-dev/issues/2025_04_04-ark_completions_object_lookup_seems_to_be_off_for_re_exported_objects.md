# Ark: Completions object lookup seems to be off for re-exported objects

> <https://github.com/posit-dev/ark/issues/764>
>
> * Author: @DavisVaughan
> * State: CLOSED
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABSPDwBw", name = "bug", description = "Something isn't working", color = "E99695")

I was typing something like `dplyr::mutate()` in a fresh session and saw this

```
[R] 2024-05-03T17:03:54.172715000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find '.data' in this environment.
[R] 2024-05-03T17:03:54.172830000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find '%>%' in this environment.
[R] 2024-05-03T17:03:54.173094000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'add_row' in this environment.
[R] 2024-05-03T17:03:54.173278000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'all_of' in this environment.
[R] 2024-05-03T17:03:54.173353000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'any_of' in this environment.
[R] 2024-05-03T17:03:54.173556000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'as_data_frame' in this environment.
[R] 2024-05-03T17:03:54.173588000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'as_label' in this environment.
[R] 2024-05-03T17:03:54.173647000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'as_tibble' in this environment.
[R] 2024-05-03T17:03:54.174188000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'contains' in this environment.
[R] 2024-05-03T17:03:54.174464000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'data_frame' in this environment.
[R] 2024-05-03T17:03:54.175025000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'ends_with' in this environment.
[R] 2024-05-03T17:03:54.175071000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'enexpr' in this environment.
[R] 2024-05-03T17:03:54.175090000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'enexprs' in this environment.
[R] 2024-05-03T17:03:54.175138000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'enquo' in this environment.
[R] 2024-05-03T17:03:54.175164000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'enquos' in this environment.
[R] 2024-05-03T17:03:54.175221000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'ensym' in this environment.
[R] 2024-05-03T17:03:54.175247000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'ensyms' in this environment.
[R] 2024-05-03T17:03:54.175334000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'everything' in this environment.
[R] 2024-05-03T17:03:54.175373000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'expr' in this environment.
[R] 2024-05-03T17:03:54.175615000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'glimpse' in this environment.
[R] 2024-05-03T17:03:54.176128000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'intersect' in this environment.
[R] 2024-05-03T17:03:54.176314000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'last_col' in this environment.
[R] 2024-05-03T17:03:54.176452000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'lst' in this environment.
[R] 2024-05-03T17:03:54.176499000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'matches' in this environment.
[R] 2024-05-03T17:03:54.177063000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'num_range' in this environment.
[R] 2024-05-03T17:03:54.177107000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'one_of' in this environment.
[R] 2024-05-03T17:03:54.177309000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'quo' in this environment.
[R] 2024-05-03T17:03:54.177341000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'quo_name' in this environment.
[R] 2024-05-03T17:03:54.177367000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'quos' in this environment.
[R] 2024-05-03T17:03:54.177919000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'setdiff' in this environment.
[R] 2024-05-03T17:03:54.177938000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'setequal' in this environment.
[R] 2024-05-03T17:03:54.178372000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'starts_with' in this environment.
[R] 2024-05-03T17:03:54.178709000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'sym' in this environment.
[R] 2024-05-03T17:03:54.178756000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'syms' in this environment.
[R] 2024-05-03T17:03:54.178868000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'tibble' in this environment.
[R] 2024-05-03T17:03:54.179073000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'tribble' in this environment.
[R] 2024-05-03T17:03:54.179102000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'type_sum' in this environment.
[R] 2024-05-03T17:03:54.179125000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'union' in this environment.
[R] 2024-05-03T17:03:54.179207000Z [ark-unknown] ERROR crates/ark/src/lsp/completions/completion_item.rs:386: Can't determine if binding is active: Can't find 'where' in this environment.
```

This seems related to re-exported functions in dplyr.

We didn't see this before because we were actually just returning if we saw a symbol that we couldn't find in the environment, but I recently switched us to log this case instead, so now we see it:
https://github.com/posit-dev/amalthea/commit/c145e9335b8d5f9c9a789d77485fd60488adfc22#diff-2498c63f0d444adeea51b3785f18f8d5f9b2531eaca916cc6f5962fa5fddadc5R374

## @DavisVaughan at 2025-04-02T15:50:05Z

`completion_item_from_symbol()` should probably just propagate an error upward (rather than logging on its own) so the caller can decide if it is really an error or just "expected" that we dont find the completion in the environment.

For example, `completion_item_from_namespace()` calls `completion_item_from_symbol()` twice, and should only log an error if it wasn't found after both calls

## @jennybc at 2025-04-02T18:32:07Z

I fell down this rabbit hole too when working on a PR (https://github.com/posit-dev/ark/pull/755/files#diff-2498c63f0d444adeea51b3785f18f8d5f9b2531eaca916cc6f5962fa5fddadc5R450-R453). It's easy to think there is a real problem and try to diagnose it, only to learn that everything is working normally. Expressed in terms of that PR:

```
unique::get_completions ->
  collect_completions(NamespaceSource, ...) ->
    source.provide_completions ->
      completions_from_namespace ->
        completion_item_from_namespace ->
          completion_item_from_symbol -> log::error!("Can't determine if binding is active: {err:?}");
          completion_item_from_symbol -> SUCCESS!
```

https://github.com/posit-dev/ark/blob/0b49cd50004a287b3b894c76f24f20bc2b54c61d/crates/ark/src/lsp/completions/completion_item.rs#L377-L398

https://github.com/posit-dev/ark/blob/0b49cd50004a287b3b894c76f24f20bc2b54c61d/crates/ark/src/lsp/completions/completion_item.rs#L440-L454

Basic problem: something that is imported then re-exported (such as dplyr does with `tibble::add_row()` or `tidyselect::all_of()`) is not found in the first environment it's sought in, so `r_env_binding_is_active()` fails and logs this error. It is subsequently found in the imports environment and all is well.
