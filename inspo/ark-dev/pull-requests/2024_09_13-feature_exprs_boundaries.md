# Detect boundaries of parse inputs

> <https://github.com/posit-dev/ark/pull/522>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Progress towards https://github.com/posit-dev/positron/issues/1326.

`parse_boundaries()` uses the R parser to detect the line boundaries of:

- Complete inputs (zero or more)
- Incomplete inputs (optional)
- Invalid inputs (optional)

The boundaries are for _lines of inputs_ rather than _expressions_. For instance, `foo; bar` has one input whereas `foo\nbar` has two inputs.

Invariants:
- Empty lines and lines containing only whitespace or comments are complete inputs.
- There is always at least one range since the empty string is a complete input.
- The ranges are sorted and non-overlapping.
- Inputs are classified in complete, incomplete, and error sections. The
  sections are sorted in this order (there cannot be complete inputs after
  an incomplete or error one, there cannot be an incomplete input after
  an error one, and error inputs are always trailing).
- There is only one incomplete and one error input in a set of inputs.

Approach:

I originally thought I'd use the parse data artifacts created by side effects to detect boundaries of complete expressions in case of incomplete and invalid inputs (see infrastructure implemented in #508). The goal was to avoid non-linear performance as the number of lines increases. I didn't end up doing that because the parse data is actually unusable for more complex cases than what I tried during my exploration.

Instead, I had to resort to parsing line by line. I start with the entire set of lines and back up one line at a time until the parse fully completes. Along the way I keep track of the error and incomplete sections of the input. In the most common cases (valid inputs, short incomplete input at the end), this should only require one or a few iterations. The boundaries of complete expressions are retrieved from the source references of the parsed expressions (using infrastructure from #482) and then transformed to complete inputs.

Supporting infrastructure:

- New `CharacterVector::slice()` method that returns a `&[SEXP]` and a corresponding `CharacterVector::TryFrom<&[SEXP]>` method. (Eventually `List` should gain those as well.) These methods make it easy to slice character vectors from the Rust side. I use this to shorten the vector of lines without having to start from a `&str` and reallocate `CHARSXP`s everytime.

- `SrcFile` can now be constructed from a `CharacterVector` with `TryFrom`.

- `Vector::create()` is no longer `unsafe`.

- `ArkRange::TryFrom<SrcRef>` method, although I didn't end up using it.

## @lionel- at 2024-09-14T11:36:52Z

TODO: 

Error section should contain line number and error message

Whitespace/comment inputs should be tagged with a boolean
