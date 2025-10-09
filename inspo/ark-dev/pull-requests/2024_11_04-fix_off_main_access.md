# Patch to avoid accessing R off the main thread in integration test

> <https://github.com/posit-dev/ark/pull/618>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Patch for https://github.com/posit-dev/ark/issues/609 which causes us some test instability. We'd like to be able to use `r_test()`s in the long term, but it requires some thinking. In the meantime we can just directly go through an execute request and use that to ensure code is executed on the main R thread.

