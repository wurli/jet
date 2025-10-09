# Move `use` calls into `mod` blocks to avoid `#[cfg(test)]` proliferation

> <https://github.com/posit-dev/ark/pull/826>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

@jennybc this solves the weirdness around having to specify `#[cfg(test)]` everywhere

In general, you put any `use` declarations that your test needs _inside the `mod tests {` block_, this ensures they are already inside the `#[cfg(test)]` of the `mod` block itself, avoiding the proliferation of `#[cfg(test)]`

## @jennybc at 2025-06-04T20:31:51Z

Oh, I know this is possible, I was just trying to de-duplicate this stuff. This is how I started out. But I really don't have a strong opinion on this one, if you prefer to handle as in this PR.

## @jennybc at 2025-06-04T20:32:46Z

I'll just merge this, since I don't have a strong opinion about the repeated `use` declarations and the rest of this I definitely want.

## @DavisVaughan at 2025-06-04T20:37:20Z

I also think it fights rust-analyzer too much to do it the other way, because the "please add a `use` directive for this" code action will put it under `mod` by default