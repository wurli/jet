# Add support for graphics engine 16 of R 4.3.0

> <https://github.com/posit-dev/ark/pull/29>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

This method was added in R 4.3:

```c
#if R_USE_PROTOTYPES
    void (*glyph)(int n, int *glyphs, double *x, double *y,
                  SEXP font, double size,
                  int colour, double rot, pDevDesc dd);
#else
    void (*glyph)();
#endif
```

