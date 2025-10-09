# Support data.tables in the data explorer

> <https://github.com/posit-dev/ark/pull/341>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/2970

The problem was that when calling `[` with a data table it quotes the `j` argument see: https://cran.r-project.org/web/packages/data.table/vignettes/datatable-faq.html#i-assigned-a-variable-mycol-quot-x-quot-but-then-dt-mycol-returns-an-error-how-do-i-get-it-to-look-up-the-column-name-contained-in-the-mycol-variable

And a similar problem to extract a column as a vector, as `data.table` simply ignores the `drop` argument.

## @DavisVaughan at 2024-05-14T19:19:11Z

@dfalbel this might need to be done in a follow up PR, but have you tested if this works?

```r
dt <- data.table::data.table(x = 1:5)

View(dt)

# new column
# likely updates the data viewer
dt[, y := 6:10]

# update existing column
# likely does not update the data viewer
dt[, x := 11:15]
```

In RStudio the "update existing column" command actually doesn't update RStudio's data viewer (even though it should)

## @dfalbel at 2024-05-14T20:06:26Z

It indeed doesn't work. We'll need to change how we detect data changes, currently we only compare the SEXP objets by address.

https://github.com/posit-dev/amalthea/blob/486dc8bfa516ea45eb4eaddb19b2128bf51f679d/crates/ark/src/data_explorer/r_data_explorer.rs#L301-L308

I'll open an issue for that.

## @lionel- at 2024-05-15T05:19:20Z

FWIW I don’t think we need to worry about updating DT mutation in place because this is undefined behaviour.

## @lionel- at 2024-05-15T05:41:43Z

> Like @lionel- said in the ark call, in vctrs we make the assumption that if you explicitly inherit from "list" then you must also be a VECSXP internally. I think we can make the same assumption here, where if you inherit from "data.frame" then you should conform to standard data frame storage rules and can be subset with the data frame [ method.

To be clear it's also a fundamental vctrs assumption for objects inheriting from "data.frame", not just for objects inheriting from "list". So we're following a strongly established convention that "data.frame" defines a storage class in addition to the data.frame interface.