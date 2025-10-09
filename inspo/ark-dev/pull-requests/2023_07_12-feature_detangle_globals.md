# Give `plots/` and `data_viewer/` their own globals separate from `lsp/`

> <https://github.com/posit-dev/ark/pull/66>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/rstudio/positron/issues/459#issuecomment-1632983087
Kind of a continuation of posit-dev/positron#50 

The `COMM_MANAGER_TX` global initialized by the LSP was being used in the data viewer code and the plots code, which:
- Doesn't feel quite right
- Was causing issues in Jupyter Notebooks, since the LSP isn't currently initialized there, so when our plot hooks try to call the globals that the LSP usually initializes, we got a segfault (see that linked comment)

I saw two ways to fix this:

One option, which I did _not_ do, was to add a top level `globals.rs` file that included globals like `COMM_MANAGER_TX` and `KERNEL_REQUEST_TX` which could be initialized early on and then shared among all of the submodules like the lsp, data viewer, and plotting code. I decided against this because:
- These globals are a hack to be able to pass them on to R callbacks, and making them into top level globals like this makes them too easily accessible, making them tempting to use in places other than R callbacks. We already use the `COMM_MANAGER_TX` global in the data viewer code, when I don't think we need to / should.
- It makes it hard to figure out where the global is coming from. i.e. you'd see a global being used in an LSP callback but you wouldn't see any LSP initialization code that set it up

The other option, which is implemented here, is to give the data viewer and plots modules their own `globals.rs` files, which:
- Makes it clear which minimal set of globals are required by each module
- Makes it (slightly) harder for other modules to accidentally use the globals
- Allows each modules globals to be initialized separately, which fixes the original issue because I can initialize the plot related globals even when the LSP isn't started.

We no longer crash when calling `plot()` in a Jupyter Notebook (yay!) but we still don't show any actual plots (boo).

<img width="1349" alt="Screenshot 2023-07-12 at 4 11 02 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/5561ddc1-e4eb-4f37-a7a9-cc1cb5af0c28">


In a future PR I'll see if I can remove the globals from data_viewer altogether, because it doesn't use R callbacks so it shouldn't need it (besides `ps_view_data_frame()`, which maybe we can remove or work around, since it is a convenience helper)

## @DavisVaughan at 2023-07-13T18:06:31Z

@lionel- I did that for the LSP and Plots modules here https://github.com/posit-dev/amalthea/pull/66/commits/b71fcdb1d94db25a5a9d8f2351737bfe7288b520 what do you think?

We can't do it for the data viewer, because it is (incorrectly, i think) using its set of globals outside of R callbacks. But I plan to remove the data viewer globals in a follow up PR, so that's fine

## @DavisVaughan at 2023-07-14T12:51:11Z

> We can't do it for the data viewer, because it is (incorrectly, i think) using its set of globals outside of R callbacks. But I plan to remove the data viewer globals in a follow up PR, so that's fine

I'm removing the data viewer globals in posit-dev/positron#67 