# Wait until 0MQ sockets are created before starting R

> <https://github.com/posit-dev/ark/pull/43>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Using a one-off channel that is activated after all 0MQ sockets are created.

Addresses rstudio/positron#720. I could no longer reproduce the crash using this branch.

