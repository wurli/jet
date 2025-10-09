# Fix indentation with `OnTypeFormatting` method for `\n`

> <https://github.com/posit-dev/ark/pull/329>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Adds an LSP method for `OnTypeFormatting` to correct the indentation of the frontend.

Addresses https://github.com/posit-dev/positron/issues/1880
Addresses https://github.com/posit-dev/positron/issues/2764
Addresses https://github.com/posit-dev/positron/issues/2707

We currently don't enable `OnTypeFormatting` per se but instead require a custom request that contains a versioned document. That is to prevent any data corruption that might result from the very unfortunate fact that our LSP server does not call handlers in message order (see https://github.com/posit-dev/positron/issues/2692#issuecomment-2075250072). The document version allows us to detect that we are working with an outdated version and decline to format in this case. This is not ideal but will prevent corruption of user files.

In the future, indentation will be provided as part of our formatter for R code. This will result in indentation that is fully consistent with our future formatter. For now the indentation approach taken here is similar to the tree-sitter indentation engine of Emacs. We provide the tree-sitter parent node of the beginning of line to an algorithm made of successive rules. These rules return an anchor node from which we indent, as well as the indentation width to add to the beginning of this node. For instance, for the indentation of top-level expressions, the anchor is the "program" node and the indentation width to add is 0. For a continuation in a chain of operators, the anchor is the topmost binary operation and the indentation is `tab_size`.

The formatting options come from two sources:

- The user/workspace settings in Positron. To support this we now have the infrastructure for watching over Code/Positron settings. These indentation settings are rich and allow for instance the tab size to differ from the indentation size, as you would find in the R core sources.

- The formatting options from the format request. These settings are more accurate because they reflect the state of the editor, which might have different settings than the current user/workspace settings if the user has changed them via the status bar in Positron. However they don't support different tab and indent sizes. If the settings are configured with differnt sizes, we just ignore the formatting options. This way we still provide decent support for this peculiar setting.

The frontend side has comprehensive integration tests but I added a bunch of unit tests in Ark too. These are supported be a new util `apply_text_edits()` that should be useful elsewhere too.


