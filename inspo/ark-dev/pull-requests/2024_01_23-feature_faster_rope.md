# Utilize `Rope` to its fullest extent!

> <https://github.com/posit-dev/ark/pull/211>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/505 (both issues brought up there are solved here)
https://github.com/posit-dev/amalthea/pull/209 was merged into this

The goal of this PR is to make our usage of `Rope` much more efficient. `Rope`s store our document text in an efficient "chunked" format, but currently at most LSP calls we are forcing the `Rope` to materialize the entire document text into a contiguous string, basically defeating the whole purpose of it! There are two places in particular that this really stinks:
- Every `did_change()` call, i.e. whenever the user types anything. I imagine that this could be pretty painful in a very large document. We currently call tree-sitter's `parse()` function to reparse the doc, which requires a contiguous string.
- Any time we call `node.utf8_text(source: &[u8])` to extract the text for a particular node, which we do quite a bit across many LSP methods. We typically make this a little more efficient by converting the document to a contiguous format once at the start of the LSP call, and storing it in something like `DocumentContext` which gets passed around and reused.

I have managed to avoid materializing the `Rope` in both of these cases:
- We now use tree-sitter's `parse_with()` alongside a parse callback that is highly compatible with Rope's "chunk" based internals. This doesn't require a contiguous buffer, and from what I can tell it was designed with this purpose in mind.
- We utilize `Rope`'s `get_byte_slice()` method to efficiently extract a byte slice out that corresponds to a `Node`'s range, and then `to_string()` that to get the text. This replaces `node.utf8_text()`.

