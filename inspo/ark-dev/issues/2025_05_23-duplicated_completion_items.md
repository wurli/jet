# Duplicated completion items

> <https://github.com/posit-dev/ark/issues/813>
> 
> * Author: @jennybc
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABwXx7Aw", name = "area: language server", description = "", color = "C2E0C6")


Noticed when writing up #812.

The object `a_kinda_long_name` appears 3 times in this completion list:

<img width="606" alt="Image" src="https://github.com/user-attachments/assets/c4306e84-f34e-4944-ba62-260065dabca4" />

Notice the three different icons. This means there are 3 different `CompletionItems` for this object, each of a distinct kind. This has become possible recently, because we now intentionally create multiple completion items of different kinds (e.g. keyword and snippet) for selected keywords (see #782).

This bug pre-existed #782, it was just hidden by the fact that one of these completion items would get there first and prevent the other two from ever making it into the list. We used to key effectively on `label` and now we key on `label` and `kind`.

I temporarily added some logging and here's a sketch of what's happening:

* First, the search path source nominates `a_kinda_long_name`, with kind 'Struct'.
* Then, the document source nominates it again, with kind 'Variable'.
* Finally, the workspace source nominates it a third time, with kind 'Value'.

I imagine the diversity of kinds here is just an accident(?). Copilot thinks "we need a shared helper function that all sources can use to determine the appropriate `CompletionItemKind` for R objects" and I have to admit that sounds like a very good idea.

---

The logging revealed some info on another duplication situation that I already knew about but hadn't studied yet. Just dumping the juicy bits from the log here so I can come back to it. This is all happening within the search path source:

```
Same label 'pak' but different kinds: existing 'Function' from source search_path vs new 'Module' from source search_path
Same label 'reprex' but different kinds: existing 'Function' from source search_path vs new 'Module' from source search_path
Same label 'zip' but different kinds: existing 'Function' from source search_path vs new 'Module' from source search_path
Same label 'class' but different kinds: existing 'Function' from source search_path vs new 'Module' from source search_path
Same label 'grid' but different kinds: existing 'Function' from source search_path vs new 'Module' from source search_path
Same label 'methods' but different kinds: existing 'Function' from source search_path vs new 'Module' from source search_path
```

Update after scrutinizing the list just above: these are names that refer to both a package and a function! So it's correct behaviour.

## @kevinushey at 2025-05-23T17:38:30Z

For what it's worth -- I think it's expected that different completion sources could produce the same completion values; it's worth having a de-duplication (or merging) step that joins completions which produce identical inserted text.

We do this in RStudio as well, e.g. a "context" completion (what variables are in lexical scope?) and a "workspace" completion (what variables are available in the session?) may produce the same completion results.

## @DavisVaughan at 2025-05-23T18:31:09Z

@kevinushey we do this, but our "key" for determining if something is the same is `name+kind`, and in this case the `name` is the same but there seem to be 3 disparate `kind`s, which really should probably all be the same `kind` if possible (it is likely fixable here, but idk).

We will probably have trouble making the `kind` consistent everywhere though. For example, search path completions have access to R itself and can do deep analysis of the object in question to come up with a precise `kind`. But for document/workspace completions that's just static analysis, so there are limitations to how exact we can be with the `kind`. Those differences will result in duplication with our current key approach, so we may need something different

As an example

```r
object <- list(fn = function() {})
fn <- object$fn
```

It's likely that static analysis just thinks `fn` is some "value" kind, but session analysis could figure out `fn` is actually a "function" kind (Yes, deep static analysis could possibly figure this case out too, but probably not in the general case)

Rather than deduplication, it's possible we are just going to want a "layering" approach keyed by just `name` where `workspace < document < session`, allowing, say, the session level `fn` to override a document level `fn`, because session is more accurate

## @jennybc at 2025-05-23T19:14:27Z

Continuing to think out loud and explain myself better:

There are legit reasons to have multiple `CompletionItem`s with the same name (or label or insert text) but with different kind. Above we found some examples of that:

* "pak" is a package and a function in the pak package. Ditto for "reprex" and "grid".
* "zip" is a package and a function in the utils package and a function in the zip package 😬
* "class" is a package 😲 and a function in the base package
* "methods" is a package and a function in the utils package

All of these really should exist with kind "Function" and with kind "Module".

There are also legit reasons to have multiple `CompletionItem`s with the same name or label and different insert text and, therefore, kind. For example, we now have `if` as a bare keyword and we also have a snippet for `if` that inserts a whole template. See #779 for more of the same.

Examples like ☝ are why we're now using label+kind as the key in the completion items map, instead of the label alone. We can't use insert text as the key, because a `CompletionItem` can have a text edit, instead of insert text, and I think we'll be making use of that soon.

The infelicity this issue targets is when we represent the same thing with multiple `CompletionItems`, e.g. the `a_kinda_long_name` in the original example.

As @DavisVaughan points out, the different sources have access to different info. So we might need a way to capture that one source (search path) should be regarded as more reliable than another (document or workspace).

Also, some kinds are always known (like "Keyword" or "Snippet", I think), as opposed to others that can be the result of a "best guess" (this whole "Struct" / "Variable" / "Value" / whatever business).
