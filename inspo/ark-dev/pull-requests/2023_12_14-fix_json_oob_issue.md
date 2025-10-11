# Fix JSON OOB issue

> <https://github.com/posit-dev/ark/pull/182>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

See also, https://github.com/posit-dev/positron/issues/1947

Currently, this test:

https://github.com/posit-dev/amalthea/blob/7ac3487613d0562b70554c6380f137231a46a431/crates/harp/src/json.rs#L579-L590

throws an R error:

```
Error: attempt to set index 3/3 in SET_STRING_ELT
```

This is due to an off by one issue when converting from JSON to R. I've fixed that here.

After fixing the issue, the 2nd test there _still_ failed with something like:

```
thread 'json::tests::test_r_to_json_objects' panicked at 'assertion failed: `(left == right)`
  left: `"list(foo = \"bar\", baz = \"quux\", quuux = FALSE)"`,
 right: `"list(baz = \"quux\", foo = \"bar\", quuux = FALSE)"`', crates/harp/src/json.rs:410:9
```

It turns out that the JSON `Map`s that serde-json produces do not preserve the insertion ordering by default. You have to enable this feature with `features = ["preserve_order"]`. I've done this everywhere we use serde-json, because I think we are going to always want this.

