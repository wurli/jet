# Detangle modules/help/errors from the LSP

> <https://github.com/posit-dev/ark/pull/50>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

A step towards https://github.com/rstudio/positron/issues/459

The LSP seems to have been in charge of initializing the public/private R modules, which meant it was also in charge of starting the help proxy server and the global error handler setup, since those two must come after the setup of the R modules.

It doesn't seem like it makes much sense for the LSP to be in charge of these, and we've got some special code in place to ensure that these pieces are only initialized _once_, while the LSP itself can be torn down and rebuilt several times as needed, which further suggests they don't belong here.

Additionally, a Jupyter notebook session won't ever start the LSP - as of right now it is a Positron specific comm. That means that Jupyter notebooks won't have our custom error handler installed, and they won't have `options(device =)` hooked up, and they won't have access to any `.ps_*()` functions.

---

This PR does a little rearranging to move the initialization of the public/private R modules, the help proxy server, and the global error handler setup all into `setup_r()`.

This is somewhat nice because it ensures that it only happens once, so we don't need that `lsp_initialized` boolean anymore.

I've pulled `errors.rs`, `help_proxy.rs`, and `modules.rs` up out of `lsp/`, along with `browser.rs` since that seems to have just been supporting `help_proxy.rs`. I've also shifted the whole `modules/` folder up one level out of `lsp/` as well - this holds the public/private `.ps_*()` R functions 

---

This seems to have worked fairly well, and running `globalCallingHandlers()` within a Jupyter notebook does now show our error handler (although errors themselves don't seem to be passed through correctly to the jupyter notebook. seems like a separate issue).

Let me know if I've missed something subtle!

## @DavisVaughan at 2023-06-22T22:57:00Z

With or without this change, you get the behaviors described below, so I don't think this PR changes anything with how we debug!

---

If there is a syntax error _before_ ark is initialized, then you get a panic due to this `unwrap()`. I think the panic is reasonable since this is during the first setup, maybe we could explicitly call `panic!()` though with a message to make it clear that we've actively thought about this case.
https://github.com/posit-dev/amalthea/blob/e12608b7c48ab2298ea2d869499e491c0964d581/crates/ark/src/lsp/modules.rs#L180

<img width="923" alt="Screen Shot 2023-06-22 at 6 43 57 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/d5650b58-b008-4d06-9caa-c8976acca89c">

If there is a syntax error _after_ ark is initialized, i.e. during the hot reload, then there is better behavior where the previous version of the function is retained and the error is just logged instead. That's due to this slightly smarter handling of a possible import error:
https://github.com/posit-dev/amalthea/blob/e12608b7c48ab2298ea2d869499e491c0964d581/crates/ark/src/lsp/modules.rs#L96C1-L98

<img width="985" alt="Screen Shot 2023-06-22 at 6 52 36 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/d9990358-a415-4b10-8a99-34fbc72ff817">

