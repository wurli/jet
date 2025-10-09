# Rationalize completion treatment of reserved words

> <https://github.com/posit-dev/ark/issues/779>
> 
> * Author: @jennybc
> * State: CLOSED
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABwXx7Aw", name = "area: language server", description = "", color = "C2E0C6")

Let's analyze the current treatment of [R's reserved words](https://rdrr.io/r/base/Reserved.html) in ark's completion system.

Three completion sources are relevant:

### Keywords

Explicit inclusion of a **subset** of R's reserved words. See [`keyword.rs`](https://github.com/posit-dev/ark/blob/c8d465018909fb9848d372f9cb355c423a030dd7/crates/ark/src/lsp/completions/sources/composite/keyword.rs#L39-L54). The main point here is (I think) setting the completion item *kind* in such a way that the UI gives the item a special icon and annotates it with `[keyword]` as opposed to `{base}`.
    
<img width="527" alt="Image" src="https://github.com/user-attachments/assets/c2d8baf4-9504-4f02-b961-d423b83735e0" />

### Search path

Generates completions by recursively walking the search path, which includes the base package, by definition. But certain reserved words are explicitly **excluded**. I think the original intent here is to exclude words that are being handled by another source. See [`search_path.rs`](https://github.com/posit-dev/ark/blob/c8d465018909fb9848d372f9cb355c423a030dd7/crates/ark/src/lsp/completions/sources/composite/search_path.rs#L55-L57).

The exclusion list is stored in the variable `R_CONTROL_FLOW_KEYWORDS`, which suggests the main targets are words related to control flow. If not excluded, these words would bubble up from the base namespace. Side observation: `function` and `return()` (not a reserved word) were recently removed from this exclusion list (https://github.com/posit-dev/ark/pull/768). Therefore `function` and `return()` now appear in search path completions and are marked with the `{base}` namespace.

### Snippets

This source is generated from a file of built-in snippets that lives in ark at [`r.code-snippets`](https://github.com/posit-dev/ark/blob/c8d465018909fb9848d372f9cb355c423a030dd7/crates/ark/resources/snippets/r.code-snippets). These generally insert a whole template. E.g., for `if` or `else` or `for`, the snippet adds any necessary parentheses/brackets and controls placeholder traversal and position of cursor at exit.

It's important to distinguish between the _snippet completion source_ and just the general idea of snippets. The snippet completion source does not have a monopoly on snippet completions. Any completion item can insert plain text, a snippet, or a text edit.

The file of built-in snippets originally lived in positron-r, which is more in line with VS Code conventions around snippets. The file was moved into ark to confer the ability to _not_ display snippets in obviously inappropriate contexts, such as completing data frame column names.

## Reserved words and coverage by different sources

| Reserved word | keyword | search_path | r.code-snippets |
|----|----|----|----|
| `if` | ❌ | ❌ | ✅ (prefix: "if") |
| `else` | ✅ | ❌ | ✅ (prefix: "el") |
| `repeat` | ❌ | ❌ | ❌ |
| `while` | ❌ | ❌ | ✅ (prefix: "while") |
| `function` | ❌ | ✅ | ✅ (prefix: "fun") |
| `for` | ❌ | ❌ | ✅ (prefix: "for") |
| `in` | ✅ | ❌ | ❌ |
| `next` | ✅ | ❌ | ❌ |
| `break` | ✅ | ❌ | ❌ |
| `TRUE` | ✅ | ❌ | ❌ |
| `FALSE` | ✅ | ❌ | ❌ |
| `NULL` | ✅ | ❌ | ❌ |
| `Inf` | ✅ | ❌ | ❌ |
| `NaN` | ✅ | ❌ | ❌ |
| `NA` and all type-specific variants | ✅ | ❌ | ❌ |

I have a few proposals on rationalizing the completion treatment of reserved words:

-   Every reserved word should have a completion item contributed by either the search path source or the keyword source. Probably by the keyword source. No reserved word should be covered only (or even at all?) by a snippet.
    -   The biggest violation of this currently is `repeat`, which is not contributed as a completion item at all! I feel like there's an implicit contract where we promise to expose the entire "base language" in completions. In practice, I believe a typical user assumes that the completion list enumerates "everything", up to any expected filtering based on context.
    -   `if`, `while`, and `for` are currently *only* covered by the snippet source, which also feels odd. I propose we move these into the keyword source (`else` is already handled there). We can still get the snippet-y behaviour of inserting a larger construct and leaving the cursor in a specific location.
-   I'm now convinced there should be no built-in snippets. See <https://github.com/posit-dev/positron/issues/7234> and #780 for more.
    -   To the extent that we like what the snippets do for certain keywords, we can achieve that by giving them the usual snippet treatment in the keyword source. I think `function` is a great candidate for a treatment that is more clever/useful than the current snippet (e.g. <https://github.com/posit-dev/positron/issues/3649>).
-   We should move `function` to the keyword source (as opposed to letting it bubble up in the search path source).
-   The `else` snippet has prefix (label) `el` and is preventing the creation of a completion item for `el()` in the methods package. If we got rid of built-in snippets and handled `else` in the keyword source, this problem goes away.

