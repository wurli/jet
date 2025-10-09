# Be defensive about global aux channel being dropped

> <https://github.com/posit-dev/ark/pull/626>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Follow-up to https://github.com/posit-dev/ark/pull/617
Addresses https://github.com/posit-dev/positron/issues/5321

This will still cause weirdness as duplicate LSP sessions will keep using that global channel to communicate log messages and diagnostics, and these will end up being published by the newer LSP session.

To fix this we could take this opportunity to move from tower-lsp to lsp-server. The former has been unmaintained for a while and has multiple issues that require workarounds. The latter would give us much more control over the LSP event loop and allow us to initiate a session shutdown from the server to prevent duplicate LSP sessions.

