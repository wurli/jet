# Various diagnostic improvements

> <https://github.com/posit-dev/ark/pull/76>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

There were some hard coded magic `kind_id()` values that we pull from here:
https://raw.githubusercontent.com/r-lib/tree-sitter-r/next/src/parser.c

We used 72, 73, 74 but in that generated file those are off by 1

```
  sym_comma = 72,
  sym__RBRACE = 73,
  sym__RPAREN = 74,
  sym__RBRACK = 75,
```

I've removed the performant "early exit" that matched on the numeric kind-id in favor of the slower match on kind, with a note about hopefully improving this later on.

---

Unfortunately it just reveals that we have more issues with this check...

We've gone from this:

<img width="462" alt="Screenshot 2023-08-14 at 4 55 11 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/b4669f5d-1f4d-4cb8-92c6-5ba515c3af6d">

To this:

<img width="501" alt="Screenshot 2023-08-14 at 4 52 54 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/b85eec76-58e6-431b-9ad0-764bb75e8529">



## @kevinushey at 2023-08-14T23:14:47Z

LGTM!

There have been a couple recent changes to the grammar, as per https://github.com/r-lib/tree-sitter-r/commits/next -- I wonder if one of these changes might unintentionally be the culprit? We could try using the different commits by changing this line:

https://github.com/posit-dev/amalthea/blob/c8327bab62a1fc4fc3d2e36c110853d4f411f6c5/crates/ark/Cargo.toml#L43

to instead use `rev = <commit hash>`.

## @DavisVaughan at 2023-08-15T14:01:04Z

I went back to this November 2022 commit
https://github.com/r-lib/tree-sitter-r/commit/4f9bcd4e750b6b53f44a69e4454d3cf2ac6adef4

Still see the paren message plus a few additional ones

<img width="480" alt="Screenshot 2023-08-15 at 9 59 05 AM" src="https://github.com/posit-dev/amalthea/assets/19150088/27e77813-e722-44f2-ad3d-3c456b32c7db">



## @DavisVaughan at 2023-08-15T16:55:19Z

@kevinushey the commits are intentionally self contained here, so hopefully that helps

---

Fixing the `node` kind issue revealed a few more. Mostly due to the fact that iterating over something like `{ x }` previously recursed over `{`, `x`, and `}` independently because of the usage of `.children()`

`{` and `}` are _anonymous_ based on how we define them in the grammar:
https://github.com/r-lib/tree-sitter-r/blob/014716ef9029dd7d270c15f02a029a9ce769cbb5/grammar.js#L264-L268

So really we only want to recurse over the _named_ nodes here.

This problem occurred for parenthesis chunks like `(x + 1)`, function parameter lists, if blocks, while blocks, and normal braced blocks. But I think I've mostly fixed them all.

At this point, none of this code throws any diagnostics anymore:

```r
fn <- function(a, b) {

}

x <- 2
(x + 1)

if (x < 5) {
    y <- 4
}

x <- 2
while (x < 5) {

}
```

I opened quite a few rlang R files and tested the diagnostics there and they look pretty good, so this is probably a good place to stop for alpha

## @lionel- at 2023-08-16T08:11:49Z

> This change was made by @lionel- in https://github.com/r-lib/tree-sitter-r/commit/1fde02c7342ad140c61f7658fcdc2cf82ebd0fa3. @lionel-, do you recall the original motivation for that change? Would you be comfortable rolling it back?

IIRC this simplification aligned tree-sitter-r with other established parsers? It's been a while since I worked with tree-sitter, and I only worked with it this one time, so I don't any particular thoughts about what is appropriate or better right now. Feel free to go either way :)
