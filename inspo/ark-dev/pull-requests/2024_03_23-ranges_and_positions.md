# OpenRPC for cursors/ranges rstudioapi shims

> <https://github.com/posit-dev/ark/pull/275>
> 
> * Author: @juliasilge
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/1312 and goes along with https://github.com/posit-dev/positron/pull/2519/

I plan to do these in this PR:

- [`insertText()`](https://rstudio.github.io/rstudioapi/reference/rstudio-documents.html) / [`modifyRange()`](https://rstudio.github.io/rstudioapi/reference/rstudio-documents.html)
- [`setCursorPosition()`](https://rstudio.github.io/rstudioapi/reference/rstudio-documents.html) / [`setSelectionRanges()`](https://rstudio.github.io/rstudioapi/reference/rstudio-documents.html)


## @juliasilge at 2024-03-28T02:38:33Z

You can test this out in an R file with some code/comments/etc in it:

```r
## set selections:
rstudioapi::setCursorPosition(rstudioapi::document_position(5, 1))
rstudioapi::setSelectionRanges(list(rstudioapi::document_position(10, 1)))
rstudioapi::setSelectionRanges(rstudioapi::document_range(rstudioapi::document_position(1, 1), rstudioapi::document_position(2, 2)))

rgs <- list(
    rstudioapi::document_range(rstudioapi::document_position(1, 1), rstudioapi::document_position(2, 2)),
    rstudioapi::document_range(rstudioapi::document_position(3, 8), rstudioapi::document_position(3, 10)),
    rstudioapi::document_range(rstudioapi::document_position(10, 1), rstudioapi::document_position(10, 2))
)
rstudioapi::setSelectionRanges(rgs)

## modify text:
rstudioapi::insertText(c(1, 1, 1, 1), "# Howdy, folks!\n")
rstudioapi::insertText(Map(c, 1:5, 1), "# ")
rstudioapi::insertText(Map(c, 1:5, 1), paste0("#", sample(letters, 5), " "))
rstudioapi::modifyRange(c(1, 1, 1, 1), "# Apple\n")
rstudioapi::modifyRange(c(1, 1, 2, 1), "# Howdy, folks!\n")
rstudioapi::modifyRange(rstudioapi::document_position(14, 1), "# Banana\n")
rstudioapi::modifyRange(list(rstudioapi::document_position(14, 1)), "# Strawberry\n")
rstudioapi::modifyRange(rgs, "# Potato!!!\n")

## at the current selection(s):
rstudioapi::insertText("# Howdy, folks!\n")
```