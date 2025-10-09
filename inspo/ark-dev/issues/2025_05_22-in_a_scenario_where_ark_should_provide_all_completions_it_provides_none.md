# In a scenario where ark should provide all completions, it provides none

> <https://github.com/posit-dev/ark/issues/770>
> 
> * Author: @jennybc
> * State: CLOSED
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABwXx7Aw", name = "area: language server", description = "", color = "C2E0C6")

The most basic gesture to get completions in VS Code / Positron is Ctrl + Space. From the [IntelliSense docs](https://code.visualstudio.com/docs/editing/intellisense#_intellisense-features):

> You can trigger IntelliSense in any editor window by typing ⌃Space or by typing a trigger character ...

(BTW this is invoking the command `editor.action.triggerSuggest`.)

Now, more often, you type a few characters and *then* you get completions.
Either automatically after 3 characters or because you request with tab or because you trigger with a character, like `$` or `@`.

But what if you want to trigger completions *before* you've typed anything? You can use
Ctrl + Space to do it.

This should reveal "all completions", for some definition of "all".
But currently ark returns ZERO completions in this scenario.
This causes VS Code / Positron to fall back to this weird language agnostic word based completion list (quote is from Intellisense docs):

> VS Code supports word-based completions for any programming language but can also be configured to have richer IntelliSense by installing a language extension.

Empirically, this list seems to consist of tokens parsed out of ?all the files you have open in the current window?. Something like that.

Ways to experience this:

1. Ctrl + Space at the R prompt in the Console

    <img width="479" alt="Image" src="https://github.com/user-attachments/assets/9f8bafce-c434-4f75-993e-1322fe9bde08" />

1. Ctrl + Space in an empty R file (looks same as above)
1. Ctrl + Space on an empty line of a non-empty R file

    <img width="517" alt="Image" src="https://github.com/user-attachments/assets/baea9a85-748d-4315-bad1-9679692e3391" />

Why does this happen?
Because the nominal completion node when the user hasn't typed anything yet does not meet our criteria for providing completions.
Mostly because our logic doesn't account for the possibility that there really is no completion node and, instead, always latches on to something "nearby", in `find_closest_node_to_point()`:

https://github.com/posit-dev/ark/blob/4074f5112b0868b1d6c14a8a42a16f0927ecf6ba/crates/ark/src/lsp/document_context.rs#L28

We then fail to pass the `is_identifier_like()` test when getting composite completions, which means we never consult keywords, snippets, the search path, the current document, or the current workspace.

https://github.com/posit-dev/ark/blob/4074f5112b0868b1d6c14a8a42a16f0927ecf6ba/crates/ark/src/lsp/completions/sources/composite.rs#L58-L60

And thus ark returns zero completions and the fallback list kicks in.
In the log, you can see the completion search just ends abruptly.

```
[R]   2025-04-10T15:47:27.133734Z  INFO  Getting completions from composite sources
[R]   2025-04-10T15:47:27.133751Z  INFO  Trying completions from source: call
[R]   2025-04-10T15:47:27.133796Z  INFO  Trying completions from source: pipe
[R]   2025-04-10T15:47:27.133816Z  INFO  Trying completions from source: subset
```

This happens in two different ways:

* The document really is empty, so the node type is 'Program' and the node text is the empty string. Cases 1 and 2 in the list above. How it looks in the log:

      [R]   2025-04-10T15:45:52.401668Z  INFO  provide_completions() - Completion node text: '', Node type: 'Program'

* The document is not empty, so `ast.root_node().find_closest_node_to_point(point)` latches on to a bit of code elsewhere in the document. The node text varies, of course, but generally it fails the `is_identifier_like()` test. Case 3 in the list above. Examples of how this looks in the log:

      [R]   2025-04-10T15:47:27.133082Z  INFO  provide_completions() - Completion node text: '10', Node type: 'Float'

      [R]   2025-04-10T16:40:39.468395Z  INFO  provide_completions() - Completion node text: ')', Node type: 'Anonymous(")")'

      [R]   2025-04-10T17:19:24.813969Z  INFO  provide_completions() - Completion node text: '
      [R]
      [R] toupper(month.abb)
      [R] ', Node type: 'Program'

Thoughts about how to solve this:

* Add the empty string or the 'Program' node as a special case that satisfies `is_identifier_like()`. See PR #772 for this. This feels like a bandaid. But it is a really good bandaid that fixes one route into this problem.
* Recognize it's possible that the completion node is actually undefined and account for this when constructing and processing a `DocumentContext`. This is a bigger undertaking.

I need to discuss with others before moving forward.

This analysis has brought up some other ideas:

* Store the the completion node's kind and text in the `CompletionContext`, alongside features like parameters hints and pipe root. You end up wanting to know these facts in various places downstream, for logging and branching, and it's a bit of a PITA to inline that repeatedly.
* Maybe we really should make "is identifier like" into another feature that we store in the completion context and can branch on. For example, I think we can probably skip all unique sources if that is false. Then, for composite sources, we could push that check down into the implementation of each composite source. We've discussed this before and already thought this might be a good idea.

## @jennybc at 2025-04-10T19:13:17Z

Summary motivated by pre-discussion in Slack:

The command `editor.action.triggerSuggest` should not lead to weird word-based completions, ever, in the R console or an R file.

Open question whether we want "all completions" or nothing. But either is preferable to weird stuff.

## @jennybc at 2025-04-10T20:26:48Z

For comparison with Python, the LSP provides real completions in all of these scenarios:

* at the console prompt
* in an empty Python file
* on an empty line in a non-empty Python file

They all look like this:

<img width="493" alt="Image" src="https://github.com/user-attachments/assets/4bd6b793-5dfa-4370-8b26-2046f25d2043" />

## @jennybc at 2025-04-16T21:47:16Z

Anecdotally, when asking for completions in one of these valid-but-empty contexts, different folks are getting different results at different times. I don't have complete clarity on this, but approximately:

* "No suggestions" is seen in release builds of Positron (?)
  
<img width="744" alt="Image" src="https://github.com/user-attachments/assets/1e832c95-cc24-4626-b1e8-f5ea59978bc4" />

* The word-based completions are seen in dev builds of Positron (?), as shown in plenty of screenshots above

