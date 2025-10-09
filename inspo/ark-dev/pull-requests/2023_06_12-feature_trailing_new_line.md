# Turn on preferred final new line settings

> <https://github.com/posit-dev/ark/pull/30>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Manually turning on `"files.insertFinalNewline": true` like we do in Positron itself, we just don't inherit this setting due to having our own settings file, I think. Can be removed if we do https://github.com/rstudio/positron/issues/726

I also prefer `"files.trimFinalNewlines": true`, which may cause a little noise in the short term if there are existing files with too many new lines at the end, but I am ok with that?

## @kevinushey at 2023-06-12T16:52:26Z

> I also prefer "files.trimFinalNewlines": true, which may cause a little noise in the short term if there are existing files with too many new lines at the end, but I am ok with that?

I think this is okay as well. In RStudio we hesitated to make this change because (at least in older versions of RStudio) you couldn't scroll past the "end" of the document, so you couldn't put the end of the document at the top of the viewport, which was annoying. Since VSCode supports this out-of-the-box I think we should turn it on.