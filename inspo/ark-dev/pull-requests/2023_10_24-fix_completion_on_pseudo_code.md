# More gracefully handle unknown first objects

> <https://github.com/posit-dev/ark/pull/127>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Fairly small completion related change. If you type something like `qqplot(x, |)` where:
- Your cursor is at the `|`
- `x` isn't actually a real object, i.e. it does not exist anywhere yet

Then triggering quick suggestions will not give you any `qqplot()` argument names.

We try to use the first argument to determine what S3 method we are in, if any, but that requires that the first object to actually exist, and in this case it doesn't.

We should not fall over in this case, I often sketch out code before the objects actually exist, so instead we now just treat it the same as if nothing was actually provided for that first argument.

Before

https://github.com/posit-dev/amalthea/assets/19150088/e39a2458-342d-4461-a740-f8057a7b0b1b


After:


https://github.com/posit-dev/amalthea/assets/19150088/ea52b8a5-429c-48b0-aeb0-63a0e1328a4e



