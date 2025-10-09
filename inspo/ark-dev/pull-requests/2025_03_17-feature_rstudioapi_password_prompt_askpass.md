# Set `askpass` option like RStudio

> <https://github.com/posit-dev/ark/pull/748>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

CC @juliasilge 

See `?rstudioapi::askForPassword`, RStudio sets this option so packages can do `getOption("askpass")` and have it work in a tool agnostic way

