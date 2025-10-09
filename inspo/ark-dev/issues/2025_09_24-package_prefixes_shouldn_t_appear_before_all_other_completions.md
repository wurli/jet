# Package prefixes shouldn't appear before all other completions

> <https://github.com/posit-dev/ark/issues/789>
> 
> * Author: @jennybc
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABwXx7Aw", name = "area: language server", description = "", color = "C2E0C6")

In the completion list, all of the installed packages appear in prefix form before any other completions. It's most obvious in the case of "all completions" (seen in recording below), but the phenomenon is there even when there's been filtering based on what's already been typed.

https://github.com/user-attachments/assets/c0d1a943-9594-43ec-9136-b8ae93c05123

It doesn't feel right to me that all of the entries like `{} asciicast::`, `{} askpass::`, etc. appear before any other type of completion.



## @DavisVaughan at 2025-05-02T12:47:13Z

This was purposeful actually, and I do like this behavior

https://github.com/posit-dev/ark/blob/40908a3b2ba4c54bba05940ffd536444abf598a0/crates/ark/src/lsp/completions/completion_item.rs#L148

https://github.com/posit-dev/ark/blob/40908a3b2ba4c54bba05940ffd536444abf598a0/crates/ark/src/lsp/completions/sources/composite.rs#L174-L179

Argument names, pipe completions, and data frame column names still have preference over these, but packages show up ahead of arbitrary functions, and I do like that a lot.

FWIW, I don't think we should optimize for the "user hasn't typed _anything_ case" that you show in your video. I think we should optimize more for the case where the user has typed a few characters and gets completions, in which case I find floating packages to the top quite useful (in particular, the `dev<tab>` case mentioned in the code comment)

## @jennybc at 2025-05-02T18:07:48Z

The specific kind of object that was on my mind when I wrote this was "stuff I just defined in the global environment". And I think putting packages / libraries at the top is an uncommon choice relative to other languages.

That being said, I don't have super strong feelings about this. Let's keep this open a bit as a place to note any ergonomic musings about completion sort order. But I won't try too hard to change your mind here.

## @juliasilge at 2025-05-02T19:00:20Z

I want to highlight the concern raised here, that completions about an installed package take precedence over functions from a package you've already loaded: https://github.com/posit-dev/positron/issues/7407

## @jennybc at 2025-05-02T19:16:19Z

That's a good connection to make. Apparently I had already forgotten about that issue, despite having commented on it. The pain of this is also heavily affected by how *many* packages you have installed and then how their names happen to stack up against functions you use a lot. I can see that the `select()` / selectr collision is pretty irritating.

## @tomasrei at 2025-08-14T08:49:45Z

> That's a good connection to make. Apparently I had already forgotten about that issue, despite having commented on it. The pain of this is also heavily affected by how _many_ packages you have installed and then how their names happen to stack up against functions you use a lot. I can see that the `select()` / selectr collision is pretty irritating.

Yes, very irritating ☺️

## @mns-nordicals at 2025-09-24T15:52:52Z

As the author of the issue [positron #7407](https://github.com/posit-dev/positron/issues/7407) I lend my support to change the sorting order of the autocomplete. I see the benefit of `pkg::fun` pattern as an R developer, but I think autocomplete is used more often by regular users that use regular imported functions. 

As for myself I quite like the way it's handled in Rstudio where `pkg::fun` pattern seem to be the last suggestion. I often use `{writexl}` but rarely import it. So I write "wri" -> tab -> arrow-up and I reach the bottom where it is the first suggestion. 

Well, consider this my attempt to bump up the attention to the issue. 