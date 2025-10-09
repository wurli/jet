# R: Statement range provider needs to take `column` into account

> <https://github.com/posit-dev/ark/issues/714>
> 
> * Author: @DavisVaughan
> * State: OPEN
> * Labels: 

From https://github.com/posit-dev/positron/issues/1464#issuecomment-1787666141

```r
if (TRUE) { # press CMD+Enter here
    print('yes')
}; lapply(1:5, print) # You'll jump past the lapply and it wont execute
```

I think this is a bug on our end where we aren't taking the `column` into account (I didn't think we needed to).

RStudio will run _both_ expressions in 1 `CMD + Enter` press. I'm not sure we want that. I feel like it would be better to stop at the `l` of the `lapply()`

Another probably related bug:

```r
if (TRUE) 
  1 + 1 else 2 + 2 # place your cursor on the `2 + 2` and hit CMD+Enter, it will run `1 + 1`
```

I'm not entirely sure what this should do, but the current behavior is odd (note that RStudio is also weird here and just runs the whole 2nd line, which also fails to parse)

