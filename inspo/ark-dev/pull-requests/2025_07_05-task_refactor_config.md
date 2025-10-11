# Simplify LSP settings

> <https://github.com/posit-dev/ark/pull/865>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

I think you'll like this one @DavisVaughan.

Branched from #859. I was concerned at how hard it was to add a new setting and section. I think the refactor in this PR will make things much simpler and easier:

- Add a generic `Setting` type that allows us to flatten a representation of all our settings in a flat array `SETTINGS` that is easy to loop over. This drastically simplify the updating logic where we send all keys of interest to the client in a flat array, as we no longer need any bookkeeping.

- The setting objects in this array contain the conversion-from-json logic as simple methods assigned to a closure field. The default handling is still delegated to `Default` methods in our setting types.

- Remove the split between VS Code and LSP settings. If we want to add vscode agnostic settings in the future, we could add editor aliases in the `SETTINGS` array..

## @lionel- at 2025-07-22T10:26:06Z

Sorry I totally missed we lost the doc updates when the AI moved code around!

I've restored the logic. Note that Positron/Code do not support per-document settings so we can't actually test this with our IDE. This is mostly an overcomplication for now but this will be useful with other IDEs like vim or Emacs, perhaps Zed.

I've checked that global settings are updated as expected, diagnostics in particular (which do need the refresh):


https://github.com/user-attachments/assets/7186908d-b214-4ed0-b8d3-54c552142c77

