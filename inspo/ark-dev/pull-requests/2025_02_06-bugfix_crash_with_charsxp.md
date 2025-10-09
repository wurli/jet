# Fix crash when inspecting a `CHRSXP`

> <https://github.com/posit-dev/ark/pull/701>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Adresses https://github.com/posit-dev/ark/issues/692

You should be able to create a binding containing a `CHARSXP` using eg:

```
x <- rlang:::chr_get("foo", 0L)
```

We could access the value of the CHARSXP, but it would require using unsafe `libr` in the variables pane, which we try to avoid, or export a harp function for this. Seems like it's not worth given how rare this happens, but I could implement this further if you think that's important.



