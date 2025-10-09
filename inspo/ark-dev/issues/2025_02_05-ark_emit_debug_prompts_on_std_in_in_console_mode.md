# Ark: Emit debug prompts on StdIn in console mode?

> <https://github.com/posit-dev/ark/issues/700>
> 
> * Author: @lionel-
> * State: OPEN
> * Labels: 

We've had plans to emit browser prompts on StdIn in notebook mode because the current experience is confusing: https://github.com/posit-dev/ark/issues/572.

But should we do this in console mode too? This would help with:

- https://github.com/posit-dev/positron/issues/4478

- In case of multiline expressions, avoid sending pending expressions to the browser prompt, instead emit those at the next regular prompt.

  This exploits the fact that input prompts on StdIn are nested in execute-request prompts and would make debug prompts consistent with readline prompts in that regard (see discussion in https://github.com/posit-dev/positron/issues/4901#issuecomment-2396438954).

Things to consider:

- Completions within debug prompts. Currently R completions are disabled in readline prompts but we'd want those in debug prompts.

- R code evaluated from an editor during a debug session should be sent directly to the browser prompt instead of being enqueued on the frontend's list of pending inputs.

This special behaviour could be maintained by adding a custom field to the stdin request to indicate this nested prompt is expecting R code.

Also what experience do we want for notebooks? Note that currently stdin prompts do not seem to work in Positron: posit-dev/positron#4920.

