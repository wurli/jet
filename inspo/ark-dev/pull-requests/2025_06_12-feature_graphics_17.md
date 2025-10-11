# Support graphics engine version 17

> <https://github.com/posit-dev/ark/pull/839>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Closes #838

Here is the R commit bumping to graphics engine 17:
https://github.com/wch/r-source/commit/a98d9f88c7f73e5f4da8209d9373bf4316ee2d10#diff-f28e48750631b022aa07416a102087eb10c48bbec71234002e20bc46c52599e9R2493

IIUC the changes really only come into play with the pre-existing `glyph()` callback, but no changes to its signature are required
https://github.com/wch/r-source/blob/4765fe9d29d89916db03d0f3552ce2df6fbea99f/src/include/R_ext/GraphicsDevice.h#L793-L795

i.e. in the R commit from above, you see in `cairoFns.c` that `Cairo_Glyph()` now does

```c
    /* Apply font variations, if any */
    int numVar = R_GE_glyphFontNumVar(font);
    if (numVar > 0) {
        applyFontVar(cairo_face, font, numVar, xd);
    }
```

Where `R_GE_glyphFontNumVar()` is a new C function from base R, and `applyFontVar()` is a new cairo helper, but `font` is the preexisting `SEXP` from the `glyph()` callback.

All of this means there is basically nothing to change on the ark side besides saying "we support this graphics version". We do that by:
- Copying and pasting `GEDevDescVersion16` -> `GEDevDescVersion17`
- Copying and pasting `DevDescVersion16` -> `DevDescVersion17`
- Copying and pasting `pDevDescVersion16` -> `pDevDescVersion17`
- Updating the `with_device!` macro with a version 17 branch

Here's a basic plot with R-devel that works now, previously it would crash R because it hit the `panic!` in `with_device!`:

<img width="805" alt="Screenshot 2025-06-12 at 9 54 31 AM" src="https://github.com/user-attachments/assets/d5386a22-3813-4c51-98a3-de63eddc2144" />



