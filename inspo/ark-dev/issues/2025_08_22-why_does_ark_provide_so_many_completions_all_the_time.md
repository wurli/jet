# Why does ark provide so many completions, all the time?

> <https://github.com/posit-dev/ark/issues/900>
> 
> * Author: @juliasilge
> * State: OPEN
> * Labels: 

I've been doing some debugging and exploration of completions in partnership with @vezwork, with the goal of getting completions in the visual editor working. It is very clear there is work to do in the visual editor itself, in terms of filtering and sorting completions.

However, at the same time, we are noticing something strange about the completions that ark provides in Positron for R. It seems like the set of completions provided is always basically the same, and always... almost all possible completions?

Here are some steps to see what I mean:


## Setup

Set up a dev build of Positron with a breakpoint in `src/vs/editor/contrib/suggest/browser/suggestModel.ts`, and change what happens around line 504 so that you can `await` and see the resulting `completions`, like this:

```ts
		const completions = await provideSuggestionItems(
			this._languageFeaturesService.completionProvider,
			model,
			this._editor.getPosition(),
			completionOptions,
			suggestCtx,
			this._requestToken.token
		);
```

Set the breakpoint so you can check out what gets returned in `completions`.

## Python

Explore what happens with some simple Python completions, after doing something like `import os`:

- With `os.` you get 377 completions, all of which are real things you can do in that namespace
- With `os.c` you get 16 completions (a subset)
- With `os.ch` you get 5 completions (a smaller subset)

## R

Explore what happens with some simple R completions, with just base R things:

- With `lib`, you get 2928 completions, which look like basically every single thing that could possible be provided as a completion for the packages I have installed
- With `librar`, it is still all 2928 completions
- With `lm`, it is now 2929 completions 🤪 

And so forth.

What might be going on here? This is definitely not urgent, but it can't be helping us performance-wise to be shuttling around every single completion every single time, right? Thoughts?

## @DavisVaughan at 2025-08-20T13:02:38Z

When you aren't in any special context, like inside a `""` string, then from ark's perspective anything goes in terms of providing completions.

Ark is not in charge of fuzzy filtering or sorting completions based on what the user has typed. We rely fully on the frontend for that. I think this is probably correct.

We likely do not want to reimplement some kind of competing fuzzy filtering mechanism in ark, as that would likely conflict with the UI options for fuzzy filtering that users can set on the Positron side. Instead we just send everything remotely relevant over and Positron handles it nicely (other IDEs would have their own options for filtering and sorting).

I think the actual completions themselves are quite small. When a user selects one we get a "resolve completion" request to do heavy duty work like showing help related to just that completion. So I don't _think_ performance is much of a concern.

---

I carefully outlined the completion problem for Quarto here https://github.com/posit-dev/positron/issues/1572

The main issue is that VS Code itself uses internal filtering and sorting techniques that _are not_ exposed to extensions via `vscode.executeCompletionItemProvider` for whatever reason. So Quarto is unable to show completions in the same was as VS Code / Positron itself. Which stinks!

## @DavisVaughan at 2025-08-20T13:10:13Z

I did have one other thought about completions when comparing to what RStudio does

https://github.com/posit-dev/positron/issues/4489#issuecomment-2312598211

I made the useful distinction there between the _set_ of initial completions that pop up vs the _filter_ that is applied after the initial set pops up once you start typing more characters to further narrow the selection.

RStudio and Positron have different behaviors regarding that _initial set of completions_, and after that they work pretty similarly with how they _filter_ them.

I had considered whether to implement a more RStudio like behavior for the initial set of completions in ark, but it would probably be an option? I wasn't totally sure what everyone would think of it. It did seem like a nice idea though.

This would make the filtering a little less fuzzy, because ark's initial set of completions would be based on a strict subset, and then it would be fuzzy from there (I am not totally convinced either way on whether this is good or bad, but it works well in RStudio).

## @lionel- at 2025-08-22T08:37:46Z

I agree it makes sense not to implement fuzzy filtering in Ark. But maybe it also makes sense for users not to expect any fuzzy filtering when providing a prefix? That makes it hard to search for symbols for which you don't remember the first letters though. That seems like the sort of things that could be a user option.

Edit: oh now I see that's what 4489 is about