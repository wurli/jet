# Add `LineRange` type

> <https://github.com/posit-dev/ark/pull/528>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

@DavisVaughan I didn't like using `TextRange` for lines in the end. `TextSize` tries hard to make the offset an opaque type and I needed lots of `.into()`. So I created our own type around an `std::ops::Range`. It mirrors (partially) the API of `TextRange` so that we don't need to think about how to use the range as much. If we really need genericity over text ranges and line ranges at some point we can create a trait. WDYT?

The type is implemented in a new `ark/coordinates` module, which should become a crate if also needed in the LSP (which is not the case at the moment in principle - we'll make it an LSP request but ideally it's a jupyter request).

## @DavisVaughan at 2024-09-17T18:36:43Z

I think `TextSize` making the `u32` opaque is nice if you use this data structure everywhere from the ground up

## @lionel- at 2024-09-18T06:12:59Z

> I think TextSize making the u32 opaque is nice if you use this data structure everywhere from the ground up

I can see how it makes sense with text offsets where lots of assumptions are baked in, such as line endings, encodings, etc. That makes it hard to compare with offsets coming from external sources. However in the case of lines, which is a much more straightforward property of a text, it's practical to compare them to raw numbers coming from a loop or some external source.
