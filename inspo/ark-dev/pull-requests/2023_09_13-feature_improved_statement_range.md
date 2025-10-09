# Implement full statement range provider

> <https://github.com/posit-dev/ark/pull/95>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/1259
Addresses https://github.com/posit-dev/positron/issues/1268
Addresses https://github.com/posit-dev/positron/issues/1416

A fully custom statement range provider, which allows you to run individual statements like `x <- 1 + 1` in the following:

```r
test_that("foo", {
  x <- 1 + 1
})
```

or in functions like

```r
fn <- function() {
  x <- 1 + 1
}
```

Obviously much more complicated than what we had before, but we basically knew we'd need something custom eventually anyways. I've also added a swath of tests to make us more confident about how it works going forward.

One thing worth noting is that this works purely off the row number, it ignores the column position entirely, which I think is the right call for how a statement range provider should work (we'd use the column position if we wanted something like "go to next selectable statement")

It also now skips comments

https://github.com/posit-dev/amalthea/assets/19150088/66999259-83f4-4859-ad34-d3b9e5e123b2





## @DavisVaughan at 2023-09-27T16:22:47Z

@lionel- it previously would run the entire expression rather than just `bar` if you placed your cursor on the `bar` line and I didn't think I could do anything about that, but I think I figured a way around that, and added many more tests. I also fixed a case where

```r
function()
{
  1 + 1
}
```

wasn't running the whole function if you put the cursor on `{`