# Index all assigned variables

> <https://github.com/posit-dev/ark/pull/620>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Quick fix to improve S7 workflow.
Addresses https://github.com/posit-dev/positron/issues/5274

Also a nice improvement all around as you can jump to the definition of e.g. a data frame in a script.

### Positron Release Notes

#### New Features

- All top-level variables in R files are now indexed as workspace symbols. You can now find assigned objects in the "Go to symbol in Workspace feature" and use "Go to definition" to jump from a variable name to its assignment (posit-dev/positron#5274). Repeated assignments to the same name are not indexed yet but we plan to do so in the future.

#### Bug Fixes

- N/A

