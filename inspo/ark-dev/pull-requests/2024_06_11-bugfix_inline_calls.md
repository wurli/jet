# Data explorer: Faster write_delim

> <https://github.com/posit-dev/ark/pull/394>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Addresses: https://github.com/posit-dev/positron/issues/3485

The issue is that our implementation of `write_delim` is terribly slow, as writing to a `textConnection` the way we were doing seems to be a lot slower then just writing to a tempfile and then reading it back to R. And gets exponentially slower as we increase the data.frame size. 

It seems that there's no way to pre-allocate the character buffer when using `textConnection`.

This will be potentially slower on Windows due to tempfiles actually materializing on disk. 
Another solution could be falling back to `read::format_delim()` which is ~10x faster than this current approach.

Eg:

<details>
<summary>Benchmark</summary>

```
library(withr)

write_delim1 <- function(x, delim, include_header) {
  con <- textConnection("text_val", "w", encoding="UTF-8")
  defer(close(con))
  
  utils::write.table(x, con, sep = delim, row.names = FALSE, col.names = include_header, quote = FALSE, na = "")
  paste0(textConnectionValue(con), collapse = "\n")
}

write_delim2 <- function(x, delim, include_header) {
  tmp <- tempfile()
  defer(unlink(tmp))
  
  utils::write.table(x, tmp, sep = delim, row.names = FALSE, col.names = include_header, quote = FALSE, na = "")
  # We use size - 1 because we don't want to read the last newline character
  # that creates problems when pasting the content in spreadsheets
  readChar(tmp, file.info(tmp)$size - 1L)
}

s <- sample.int(nrow(mtcars), size = 100000, replace = TRUE)
x <- mtcars[s, ]
bench::mark(
  write_delim1(x, "\t", TRUE),
  write_delim2(x, "\t", TRUE)
)
```

```
# A tibble: 2 × 13
  expression         min  median `itr/sec` mem_alloc `gc/sec` n_itr  n_gc
  <bch:expr>     <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl> <int> <dbl>
1 "write_delim1…   24.3s   24.3s    0.0411    37.3GB     23.5     1   570
2 "write_delim2… 524.2ms 524.2ms    1.91      14.7MB      0       1     0
# ℹ 5 more variables: total_time <bch:tm>, result <list>, memory <list>,
#   time <list>, gc <list>
```

</details>

Initially, I was think that it could be somehow related to https://github.com/posit-dev/ark/issues/695,
but it seems to work well now. It's probably because `write_delim()` was taking too long, then if we tried to interrupt the R kernel, it triggered a task interrupt which in turn triggered deparsing the call containing the large data.frame in it.


