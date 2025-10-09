# Fixes for folding ranges

> <https://github.com/posit-dev/ark/pull/825>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Follow up to #815.

- Fix detection of code chunks. `# %%` are now supported, `# +` is no longer recognised as chunk.
- Remove unwraps for safety.
- Fix end range of comment sections. They now extend up to the next section, including empty lines. Important because chunks do the same and they need to be nested in the comment sections. Otherwise the frontend is confused and removes the ranges that don't match its expectations.

Can you please take a look at these changes @kv9898 ?

## @kv9898 at 2025-06-05T00:21:00Z

I couldn't comment where you did not make any changes so I will just type it here. Other than the two points below and the question I raised in the review, this looks great!

1. There remain other usage of `find_last_non_empty_line`. Should we drop the use of it altogether? Otherwise a cell like this in the end would still leave an empty line in the end:

```r
# %% Something
foo()


```

2. I suggest we change lines 127-134 to:
```rust
        end_node_handler(
            document,
            folding_ranges,
            end.row + 1, // added +1
            &mut child_comment_stack,
            &mut child_region_marker,
            &mut child_cell_marker,
        );
```
as currently a document with no empty line in the end like the example below would not fold properly with our current implementation.

```r
# some header ####
a <- 1
b <- 2
```

## @lionel- at 2025-06-05T06:06:13Z

@kv9898 Thanks so much for taking a look! You're totally right, I've fixed the two issues you brought up and we now have more consistent coverage endings.