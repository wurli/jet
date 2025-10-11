# Remove `ARK_VDOCS` by sending vdocs over a Kernel -> LSP event

> <https://github.com/posit-dev/ark/pull/666>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Part of https://github.com/posit-dev/ark/issues/661

There was a TODO to do this anyways, and it seemed like a pretty reasonable alternative to the global.

Getting this right is a little tricky. It's common for the base R packages (and anything loaded in your `.Rprofile`) to have their namespace vdocs be generated _very quickly_ after startup before the LSP has even started up yet. This means we wouldn't have access to those namespace vdoc files, which means you would not be able to debug those packages.

We _could_ try and fix the timing so that we don't generate any vdocs until after the LSP starts up, but that is pretty brittle.

The alternative here is to maintain our own copy of the virtual documents on the kernel side. As soon as the LSP connects and we get `lsp_events_tx`, we send over any known virtual documents (anything we attempted to send over before this connection would have been dropped). This also has the benefit of being able to refresh the LSP if it restarts out from under us.

---

Side note: In the future I think it might be better if the kernel itself was the one registered as the `TextDocumentContentProvider` for these virtual documents. It's unlikely that the debugger will ever move out of ark, but the LSP probably will. It feels quite weird for Positron to be asking the LSP for the contents of one of these documents. I wonder if instead our `VirtualDocumentProvider` on the positron-r side can perform a Jupyter request to ark for the document contents?

