# Only force ALTREP compact row names (i.e. from duckplyr) if requested

> <https://github.com/posit-dev/ark/pull/745>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/4158 at the request of @hadley (but not the part about errors during materialization crashing the kernel, that is a much deeper and harder to fix problem that requires us to adjust our assumptions about what a simple `INTEGER_ELT()` call can do - i.e. in an ALTREP world that can run arbitrary code that can error 😢)

Joint PR with https://github.com/tidyverse/duckplyr/pull/661

The problem is that duckplyr uses ALTREP compact row names as the `R_RowNamesSymbol` attribute. Calling `INTEGER_ELT()` or `INTEGER()` on this to get the number of rows will trigger the entire duckdb query to run, something we want to avoid. This is hard though, because that's how we determine the number of rows in the data frame. We've determined that rather than trying to carefully avoid ALTREP compact row names every time we look at the number of rows in a data frame, a better solution is to use @dfalbel's hooks for allowing S3 classes to provide custom Variables Pane implementations - avoiding these troublesome code paths entirely.

Most of the work that was _required_ for this is actually in https://github.com/tidyverse/duckplyr/pull/661. However, in `variable_length()` here on the Ark side we were calling `table_info()` which queried the number of rows (bad) and number of columns even though we only needed the number of columns. I've reworked some things to expose ways to compute _just_ the number of columns or _just_ the number of rows for a data frame. With that in place, https://github.com/tidyverse/duckplyr/pull/661 does the rest of the work.

This won't trigger the duckdb query:
- Creating a duckdb tibble and having it show up in the Variables pane

This will trigger the duckdb query, and we are ok with this, and this is similar to RStudio:
- "Expanding" the tibble in the Variables pane to look at the column values
- Viewing the tibble in the Data Explorer

Here are some videos of me toying around with this:

https://github.com/user-attachments/assets/2a82111b-5561-421e-b259-79c51fcfea11

https://github.com/user-attachments/assets/754a6ab4-8c94-4267-be4c-e46884da63bf

See also, RStudio's `rs_dim()` here:
https://github.com/rstudio/rstudio/blob/7a9ab7afe9ae006e60897596a189a904a716ec4f/src/cpp/session/modules/environment/SessionEnvironment.cpp#L536
https://github.com/rstudio/rstudio/blob/7a9ab7afe9ae006e60897596a189a904a716ec4f/src/cpp/session/modules/SessionEnvironment.R#L560


## @dfalbel at 2025-03-13T22:27:39Z

I didn't look at the code still. I wonder if it would make sense to implement duckdb specifics using the variables pane extension mechanism. For instance by implementing custom `ark_positron_variable_display_value()` for `prudent_duckplyr_df` / `duckplyr_df`. This could potentially even live in duckplyr's codebase.

## @krlmlr at 2025-03-14T16:11:57Z

Thanks for looking into it.

Calling `head()` on the duck frame (the same way pillar does) won't materialize and will be faster in many cases.

Why is it hard to handle ALTREP failures? Does this change mean that the kernel won't crash for a stingy duck frame unless the user clicks a button? Could this be worked around with calling `head()` ?

## @hadley at 2025-03-24T15:20:40Z

@krlmlr why does an ALTREP access generate an error? That seems weird to be since you'd normally expect (e.g.) `INTEGER(x)` to always work.

## @krlmlr at 2025-03-24T16:01:56Z

This may happen for any number of reasons, including out-of-memory conditions. The "prudence" feature tries to avoid these OOM conditions and makes the errors more visible.

Can you think of an alternative?