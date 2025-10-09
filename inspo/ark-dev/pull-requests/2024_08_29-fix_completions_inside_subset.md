# Implement "string subset" completions

> <https://github.com/posit-dev/ark/pull/493>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Closes https://github.com/posit-dev/ark/pull/433
Addresses https://github.com/posit-dev/positron/issues/3931

Commits are self contained and probably easiest to read on their own

https://github.com/posit-dev/ark/pull/433 moves string completions from unique -> composite, but I strongly believe that string completions are still a unique case and should not be combined with other generic completion paths.

When I set up string completions, we only had file completions as a possible variant of string completions, but I set it up to be able to extend this with other unique string completion variants. I believe that "string subsetting" is another one of these.

This new string subsetting path is different from the composite subsetting path, with the difference being:
- Unique (string subset): `x["<tab>"]` (this PR)
- Composite (subset): `x[<tab>]` (already had this)

They share a little infrastructure though, so I've extracted out a new `common/` folder to hold it. We already have a `utils.rs` file but it is getting a little messy and I think this is a better way long term.

---

It's a bit tricky to implement this, because 99.9% of the time the correct thing to show when a user does `"<tab>"` are file completions. `x["<tab>"]` is a _very_ special case where names of the `x` object are likely more relevant, so we show those instead.

I've also allowed _very_ simple `c()` calls, like `x[c("foo", "<tab>")]` to trigger these string subset completions as well. This is a heuristic @kevinushey and I talked about. I realize it is not perfect but it should capture lots of use cases.

What we don't want to do is allow string subset completions anytime we are inside a `[` or `[[`. For example, `x[read_file("<tab>")]` and `x[match(foo, "<tab>")]` are two places where completing with the names of `x` make no sense at all, and we are better served just falling back to file name completions like usual. For that reason, I've scoped this feature very tightly to only allow this with:
- "top level" strings inside a `[` or `[[`
- Simple `c()` calls

https://github.com/user-attachments/assets/de6017e0-3197-4141-abd2-dddb67929372




## @kevinushey at 2024-08-29T18:42:54Z

Very nice -- looks good to me!

My main question (which we discussed a bit already, but worth doing that once more publicly): do we want to allow for completions of row names, as RStudio does? I'm somewhat conflicted on this one. It's useful for "vanilla R data.frames, or users who are subscribing to the "base R" mindset; however, it's not so useful for tibbles or users primarily working with the tidyverse.

A compromise would be to allow row name completions for "vanilla" data.frames, but the divergence in completion behavior might be awkward as well.

I'm curious to know what others think.

## @DavisVaughan at 2024-08-29T19:22:28Z

Would we complete both column and row names for `x["<tab>"]`? Since `x["col"]` and `x["rowname",]` are both valid?

I'll be honest I've never done `x["rowname",]` in my life 😭 

## @juliasilge at 2024-08-29T19:41:05Z

Adding support for row names here seems confusing/awkward on multiple fronts, both "this is acting different now that I made my dataframe a tibble" as well as "why are my column and row names both showing up as completions at the same time". I think I would want more clear user feedback that people need/miss the rowname completions.