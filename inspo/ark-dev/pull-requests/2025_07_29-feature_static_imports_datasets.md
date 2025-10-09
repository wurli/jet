# Treat all documented names as known symbols

> <https://github.com/posit-dev/ark/pull/883>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Branched from #872.

Progress towards https://github.com/posit-dev/positron/issues/8521
Addresses https://github.com/posit-dev/positron/issues/8708

This adds all symbol-looking documentation topics to our set of known exported symbols to cover exported datasets. This information is installed in an INDEX file that we can inspect statically: https://cran.r-project.org/doc/manuals/r-devel/R-exts.html#The-INDEX-file-1. This INDEX file might be manually written by the package author (see link above), and so might throw off our attempts at parsing this unstructured file. That said I expect it's automatically generated in the vast majority of cases

This stopgap approach will potentially produce false negatives:

- Documented topics are not necessarily about exported symbols.
- This approach will indistinctly export datasets whether `LazyData` is true or false. In the false case, the dataset is only exported after a corresponding `data()` call.

For now though, we prefer to avoid false positives and spurious diagnostics.

### QA Notes

In a fresh session with:

```r
# `LazyData: true` case
penguins # Not in scope
library(palmerpenguins)
penguins

# `LazyData: false` case
liner # Not in scope
library(BCA1SG)
liner # FIXME: Should not be in scope
data(liner)
liner
```

You should only see diagnostics _before_ the `library()` calls:

<img width="359" height="274" alt="Screenshot 2025-07-29 at 16 40 02" src="https://github.com/user-attachments/assets/284ad92a-c287-4c41-a3ec-2a47d18d02a1" />


## @DavisVaughan at 2025-07-29T16:01:20Z

Reviewed together over zed