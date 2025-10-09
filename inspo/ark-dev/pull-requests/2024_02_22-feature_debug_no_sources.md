# Implement debugging fallback when there are no srcrefs

> <https://github.com/posit-dev/ark/pull/249>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

This serves as a "fallback" approach when we don't have source references. Even with the approach implemented in https://github.com/posit-dev/amalthea/pull/251, there are going to be many places where we still don't have source references, and this handles those.

---

This generally works in the following way:

- If we discover that we _don't_ have source references for a frame on the stack, then rather than returning a `path` back to the Rust side, we instead return text `contents` containing the function being debugged, along with line/column offsets into the `contents` rather than into the `path`.
- Those `contents` get stored in the `Dap` struct as a hash map of `contents -> source_reference`, where `source_reference` is an integer ID. When we send the `Source` struct to the frontend, we end up sending over the `source_reference` ID rather than the `contents` themselves.
- The above bullet is combined with the fact that we now support the DAP `Source` "command", which allows the frontend to request the `contents` for a particular `source_reference` lazily "when it needs it" (i.e. when the user tries to view that stack frame). When we get a `Source` command, we loop through the hash map values to find the `source_reference` and return its key (the `contents`).

That is generally how we support non-`path` based `Source`s. The rest of the complexity in this PR is related to building a useful `Vec<FrameInfo>` object.

When we don't have the source refs, we need to collect a few pieces of information "manually":
- The `call` that the user is currently on for this frame
- The `contents` of the function for this frame
- The `line` and `column` (and end line/column) locations of where that `call` is in the `contents`

This is relatively difficult. I'll explain how we get these pieces in the following sections, keep in mind that this is only for the cases where we don't have the source references for that frame.

---

### The `call`

For most frames, we can get the `call` from `sys.calls()`.

However, we don't get the call for the "current context" frame that the user is actively stepping at. Instead, we recover this by capturing the stdout that R produces at each step. It prints out `debug: <call>`, and we capture `<call>` and forward it along to the R side to be used as the `call` for the current context frame.

### The `contents`

For most frames, we can get the `contents` from `sys.function()`. I've wrapped that into a Rust level `r_sys_functions()` that loops over each frame and calls `sys.function(i)` to collect them all.

This works for the "current context" frame too, where the last `sys.function()` frame corresponds to the function that we are actively stepping in.

For the "top level" frame, where the user sent to the console something like `lag(1:5)`, we don't have a corresponding `sys.function()` call, so instead we just use the call of `lag(1:5)` as the `contents`. This works pretty well, as it just gives us something to put at the top level of the stack to give a full representation of "how we got here".

### The `line` and `column` info

For the top level frame, we return `0`s for this information. As discussed above, we just return the call as the `contents`, so we don't need to highlight anything about what line we are on. There's really no "stepping" to be done here.

For any other frames where we don't have the source references, we take the `contents` of that frame and the corresponding `call` and do some trickery to match the `call` into the `contents`. The way we do this is to _reparse_ the `contents` with `parse(text = contents, keep.source = TRUE)`. We then recursively iterate over the parsed result looking for `{` nodes. Each `{` node has a list of source references as an attribute, where those source references correspond to all of the possible expressions inside that `{` node. For example:

``` r
fn <- function(x) {
  1 + 1
  
  if (x > 1) {
    2
  } else {
    3
  }
  
  2 + 2
}

contents <- deparse1(fn, collapse = "\n")
cat(contents)
#> function (x) 
#> {
#>     1 + 1
#>     if (x > 1) {
#>         2
#>     }
#>     else {
#>         3
#>     }
#>     2 + 2
#> }

fn <- parse(text = contents, keep.source = TRUE)[[1]]

# Top level expressions
fn[[3]]
#> {
#>     1 + 1
#>     if (x > 1) {
#>         2
#>     }
#>     else {
#>         3
#>     }
#>     2 + 2
#> }
attributes(fn[[3]])$srcref
#> [[1]]
#> {
#> 
#> [[2]]
#> 1 + 1
#> 
#> [[3]]
#> if (x > 1) {
#>         2
#>     }
#>     else {
#>         3
#>     }
#> 
#> [[4]]
#> 2 + 2

# Each nested `{` has its own list of src refs that we can extract
fn[[3]][[3]][[3]]
#> {
#>     2
#> }
attributes(fn[[3]][[3]][[3]])$srcref
#> [[1]]
#> {
#> 
#> [[2]]
#> 2

# For example, this tells us where `1 + 1` is in the `contents` for `fn`
attributes(fn[[3]])$srcref[[2]]
#> 1 + 1
unclass(attributes(fn[[3]])$srcref[[2]])
#> [1] 3 5 3 9 5 9 3 3
```

Extracting out all of this information works fairly well. We end up with a flat list of possible expressions and their source reference locations. We convert the expressions to text, strip out all spaces and newlines (to avoid weirdness with indents and whatnot) and try an exact match of that against the `call`. If we get a hit, we return the attached source reference location.

This tends to work fairly well, as `call` is typically deparsed in the same way as `contents` so any weirdness in the deparsing process is at least likely to be consistent between the two objects. However, there are two extra things we have to account for

#### Partial matching

While extracting out source references typically works quite well, there are places where we don't get the full set of possible expressions that we can step to. One of those is `if` statements that don't use a `{` brace.

Consider the following:

```r
fn <- function(x) {
  1 + 1
  
  if (x > 1) # no `{` here
    2
  else # no `{` here
    3
  
  
  2 + 2
}
```

In this case, when we iterate over the nodes in the parsed structure, we never hit a `{` for the bodies of the if statement, so we can't extract source reference locations for `2` and `3`. They simply don't show up in the structure we walk over even though they will show up as valid places to stop at, i.e. R will show `debug: 2` if we step there.

We work around this by allowing a _partial match_ if we don't hit an _exact match_. Since we _do_ have the full `if` node, we can get a hit by looking for `2` in the collapsed if node of `if(x>1)2else3`. We count this as a match, but our `line` and `column` information here corresponds to the full if node. This ends up looking alright, we basically just highlight the same if node twice (once when you hit the if node, and the same location again when you step into it to the `2` position).

#### Multiple matches

In both the exact match and partial match cases, it is possible to get multiple hits. This is a problem, how do you know which hit to highlight?

```r
fn <- function(x) {
  1 + 1
  2 + 2
  1 + 1 # Say we see `debug: 1 + 1`, which do we highlight?
  2 + 2
}
```

The most important case where this matters is the "current context" frame where the user is typically actively stepping in. To help combat multiple matches, we keep track of the _last start line_ of the current context frame. So in the following example, we'd see that the last start line was line 3 where we previously stepped over `2 + 2`. This allows us to use this as a minimum bound to filter out matches that occurred before this. Then we just take the 1st match of those leftover.

For any other frame on the stack besides the current context frame, it proved to be a little difficult to track the last start line, and not really worth it since the user is typically only looking at the deepest frame in the stack. So in those cases we simply take the first match. We could try to improve this in the future if we wanted to, but I'm hoping this is rare to hit.

## @DavisVaughan at 2024-02-22T22:43:28Z

I want to note that the "matching" of the `<call-text>` emitted to stdout by R as `debug: <call-text>` against the function body occurs in a very different way than what RStudio does. RStudio does a purely textual match, i.e. using `gregexpr()`, plus a lot of trickery to handle indents and spaces and stuff.

We take a different approach by deparsing the function, then reparsing it again _with source references_. This allows us to then recursively step through the function body, extracting out the `srcref` lists from each `{` node in the body (think, if statements, in addition to the outer `function() {`). This gives us a near complete list of potential expressions that can show up as `<call-text>` _and_ it gives us their row/col locations relative to the function.

This seems to work fairly well most of the time. It still has some issues, like with:

```r
if (foo)
  bar
```

where the if statement doesn't have `{`. In that case, `bar` is a valid "step" that shows up as `<call-text>`, but we never see it in the srcrefs. We _do_ see the whole `if` node though, which contains `bar`, so I've got a fallback in place for this situation where we just highlight the _whole_ if statement again when we step from here:

```r
>if (foo)
  bar
```

to

```r
if (foo)
  >bar
```

it doesn't end up looking too jarring tbh (and again, this is the fallback approach, so slightly suboptimal behavior is ok).

I did start with the RStudio approach, and for posterity I am going to drop in my translation of the RStudio function that does the pure textual matching, in case we ever need it:

```r
#' Given a function and some content inside that function, returns a vector
#' in the standard R source reference format that represents the location of
#' the content in the deparsed representation of the function.
#'
#' @param fun_lines A character vector of lines broken on `\n` that represent the function
#'   to search in.
#' @param call_lines A character vector of lines broken on `\n` that represent the call to
#'   search for.
#' @param line_minimum A single integer representing the previous line used when
#'   debugging this function. Used as a minimum bound to break ties whenever the same
#'   expression appears multiple times in the same function.
locate_call <- function(fun_lines, call_lines, line_minimum) {
  # Remember the indentation level on each line (added by deparse), and remove
  # it along with any other leading or trailing whitespace.
  fun_indents <- nchar(sub("\\S.*", "", fun_lines))

  # Remove leading/trailing whitespace on each line
  fun_lines <- sub("\\s+$", "", sub("^\\s+", "", fun_lines))
  call_lines <- sub("\\s+$", "", sub("^\\s+", "", call_lines))

  # Compute the byte position of the start of each line
  # (`fun_line_start_bytes[[1]]` is the start position of line 2,
  #  `fun_line_start_bytes[[N]]` is the length of the function)
  nchars <- 0L
  fun_line_start_bytes <- integer(length(fun_lines))
  for (i in seq_along(fun_lines)) {
    nchars <- nchars + nchar(fun_lines[[i]]) + 1L
    fun_line_start_bytes[[i]] <- nchars
  }

  # Collapse into a single line for matching
  fun_collapsed <- paste(fun_lines, collapse = " ")
  call_collapsed <- paste(call_lines, collapse = " ")

  # Any call text supplied is presumed UTF-8 unless we know otherwise
  if (Encoding(call_collapsed) == "unknown") {
    Encoding(call_collapsed) <- "UTF-8"
  }

  # NULL is output by R when it doesn't have an expression to output; don't
  # try to match it to code
  if (identical(call_collapsed, "NULL")) {
    return(c(0L, 0L, 0L, 0L))
  }

  # Perform the match attempt
  match <- gregexpr(call_collapsed, fun_collapsed, fixed = TRUE)[[1]]

  if (length(match) > 1L) {
    # There is more than one instance of the call text in the function; try
    # to pick the first match past the minimum line.
    best <- which(match > fun_line_start_bytes[line_minimum])
    if (length(best) == 0L) {
      # No match past the minimum line, just pick the match closest
      best <- which.min(abs(line_minimum - match))
    } else {
      best <- best[[1L]]
    }
    start_byte <- match[[best]]
    end_byte <- match[[best]] + attr(match, "match.length")[[best]]
  } else {
    start_byte <- match
    end_byte <- match + attr(match, "match.length")
  }

  # Return an empty source ref if we couldn't find a match
  if (length(start_byte) == 0L || start_byte < 0L) {
    return(c(0L, 0L, 0L, 0L))
  }

  # Compute the starting and ending lines
  start_line <- which(fun_line_start_bytes >= start_byte)[[1L]]
  end_line <- which(fun_line_start_bytes >= end_byte)[[1L]]

  if (is.na(end_line)) {
    end_line <- length(fun_line_start_bytes)
  }

  # Get byte offset of the beginning of the start/end lines to be able to compute the
  # real column positions
  if (start_line == 1L) {
    start_line_byte <- 0L
  } else {
    start_line_byte <- fun_line_start_bytes[start_line - 1L]
  }

  if (end_line == 1L) {
    end_line_byte <- 0L
  } else {
    end_line_byte <- fun_line_start_bytes[end_line - 1L]
  }

  # Compute the starting and ending column positions within the line,
  # taking into account the indents we removed earlier.
  start_column <- start_byte - start_line_byte
  start_column <- start_column + fun_indents[start_line]

  end_column <- end_byte - end_line_byte
  end_column <- end_column + fun_indents[end_line]

  out <- as.integer(c(
    start_line,
    start_column,
    end_line,
    end_column
  ))

  return(out)
}
```

## @lionel- at 2024-03-01T10:50:50Z

> We take a different approach by deparsing the function, then reparsing it again with source references. This allows us to then recursively step through the function body, extracting out the srcref lists from each { node in the body (think, if statements, in addition to the outer function() {). This gives us a near complete list of potential expressions that can show up as <call-text> and it gives us their row/col locations relative to the function.

oh I was imagining you'd use tree-sitter for this step, in case of unparsable calls:

```
> fn <- rlang::inject(function() { browser(); !!new.env(); NULL })
> fn()
Called from: fn()
Browse[1]> debug at #1: <environment: 0x109280e88>
```

Though I guess we can't rule out weirdness for erroring parse trees. The main question is how well the TS parses preserves the `{` structure, since that's what we mainly need to match against.

Maybe we should just use the textual approach as the fallback's fallback if parsing fails :grimacing:?


> For the "top level" frame, where the user sent to the console something like lag(1:5), we don't have a corresponding sys.function() call, so instead we just use the call of lag(1:5) as the contents. This works pretty well, as it just gives us something to put at the top level of the stack to give a full representation of "how we got here".

This makes me think that sending execute requests with source info would allow us to reference the original editor file here. (And then in the future pass the information to the R parser as discussed elsewhere.) I've opened https://github.com/posit-dev/positron/issues/2364 about this.


## @DavisVaughan at 2024-03-01T14:39:15Z

> The experience of navigating non-R files is a bit subpar. It would be nice to find a solution to get a .R extension in those debugger vdocs.

My _guess_ is that after we merge in your PR on top of this, we can transition this to instead use an ark virtual document where the source request can be managed by `ark/internal/getVirtualDocument`

(i.e. we'd probably remove the support for the `sourceReference` ID and the `Source` command, and instead supply a `path` to an ark virtual document containing the function sources)

## @DavisVaughan at 2024-03-01T19:42:55Z

https://github.com/posit-dev/amalthea/pull/249/commits/de5526a604a59129aa490b96838d59a93ed5c943 adds some fallback behavior for known non parseable cases by looking at the `deparse()` source code to see when it "gives up"