# Followup to #1585 index out of range -- log an error if not already handled

> <https://github.com/posit-dev/ark/pull/118>
> 
> * Author: @jgutman
> * State: MERGED
> * Labels: 

https://github.com/posit-dev/positron/pull/1614 handles this issue on the frontend, such that R should never receive a `size` that will create an out of range situation. But just in case, we add this bit of error handling

