# Exclude comment sections from workspace symbols by default

> <https://github.com/posit-dev/ark/pull/866>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Branched from #865.
Addresses https://github.com/posit-dev/positron/issues/4886.

When a comment section like `# name ---` clashes with an actual symbol like `name <- function() {}`, whichever symbols comes first (usually the section) is included in the workspace index, and the other is ignored. Ideally we'd track both but for now we only track one identifier per file.

This causes sections to win over function definitions in cases like this:

```r
# my_section ----

my_section <- function() {}

my_section # Reference
```

If you then jump to definition on a reference to that function, Positron jumps to the section instead of the function.

* To fix this, we now allow functions and variables to overwrite sections in the indexer.

* In addition, we no longer emit sections as workspace symbols by default. This can be turned back on with a new setting `positron.r.workspaceSymbols.includeCommentSections`. This is consistent with the fix in https://github.com/quarto-dev/quarto/pull/755 where we no longer export markdown headers as workspace symbols by default, to avoid flooding workspace symbol quickpicks (`#` prefix in command palette) with section headers.


### QA Notes

- You should be able to command+quick on the `my_section` reference and be taken to the function definition rather than the section. This should be the case no matter the value of `positron.r.workspaceSymbols.includeCommentSections`.

- When `positron.r.workspaceSymbols.includeCommentSections` is set to `true`, you should see comment sections in the workspace symbol quickpick. Otherwise they shouldn't be included.

## @lionel- at 2025-07-22T14:54:07Z

We've now got the requested test coverage and a test-only RAII indexer guard to help clean up the index after tests have run.