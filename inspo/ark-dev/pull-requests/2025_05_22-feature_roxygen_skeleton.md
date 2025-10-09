# Add infrastructure for Code Actions 💡, and our first action for generating a roxygen template

> <https://github.com/posit-dev/ark/pull/809>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/1744

This PR implements our first Code Action 💡, generating roxygen documentation templates

Code Actions show up as contextual 💡 in the user's UI. I've followed the rust-analyzer model, which enables this code action when the user's cursor is _on the function's name_.
https://rust-analyzer.github.io/book/assists.html#generate_documentation_template

I hope this is the first of many 💡s that we create for users, because these are very cool!

https://github.com/user-attachments/assets/6fc9cf0d-1726-42ef-9a8a-5c708db69488

A few notes:
- The code action is _not_ enabled if the previous line starts with a `#'` already (i.e. it somewhat looks like you've already got docs)
- The code action does handle indentation, i.e. if you are for some reason documenting a function that is nested
- The code action does not handle R6 class names `Foo <- R6::R6Class()`, nor does it handle R6 methods `list(fn = function() {})`, but neither does RStudio and I think that is okay for now. We could probably handle R6 methods with a little extra effort if we feel it is important.

