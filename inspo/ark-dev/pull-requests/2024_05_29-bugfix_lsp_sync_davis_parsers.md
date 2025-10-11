# Extract out `ParserState` only used by exclusive ref handlers

> <https://github.com/posit-dev/ark/pull/372>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

To get rid of `Arc<Mutex<Parser>>` and our SAFETY note about this

It didn't seem too difficult after all, and I think it makes things a little clearer

## @lionel- at 2024-05-30T06:00:44Z

I tried this direction and then thought this was a lot of complication compared to just treating the field private as a convention. But I don't mind doing this if you prefer.
