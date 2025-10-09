# Switch GetSchema from column range to column indices

> <https://github.com/posit-dev/ark/pull/453>
> 
> * Author: @softwarenerd
> * State: MERGED
> * Labels: 

This PR switches `GetSchema` from accepting a column range (`start_index` / `num_columns`) to accepting an array of column indices (`column_indices`). This addresses a nascent bug because the Data Explorer was calculating column indices but then passing them as a column range.

This change also addresses the `i32` limitation of the previous `GetSchema` implementation. So this comment no longer applies:

```
// TODO: Support for data frames with over 2B rows. Note that neither base R nor
// tidyverse support long vectors in data frames, but data.table does.
```

It is now possible to load column indices greater than `2_147_483_647`.
It also attempts to bump the ark version to `0.1.121`.
It also adds `.DS_Store` to the `.gitignore` file.

Please do not merge this PR without also merging https://github.com/posit-dev/positron/pull/4180.

