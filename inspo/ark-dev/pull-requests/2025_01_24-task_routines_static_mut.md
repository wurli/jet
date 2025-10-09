# Fix `static mut` issues with routines

> <https://github.com/posit-dev/ark/pull/677>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Closes #661.

Ideally `R_ROUTINES` would be thread-local but that's currently inconvenient in unit tests.

