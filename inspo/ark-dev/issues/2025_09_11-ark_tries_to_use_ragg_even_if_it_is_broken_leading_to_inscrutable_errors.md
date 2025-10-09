# Ark tries to use ragg even if it is broken, leading to inscrutable errors

> <https://github.com/posit-dev/ark/issues/917>
> 
> * Author: @jmcphers
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABSPDwBw", name = "bug", description = "Something isn't working", color = "E99695")

If `ragg` is installed but unusable, then attempting to plot anything will result in an unhelpful error, `no active device and default getOption("device") is invalid`:

<img width="654" height="99" alt="Image" src="https://github.com/user-attachments/assets/2f8939c6-b2e9-4aa1-8cf2-405910a57b25" />

You can get a slightly better error by calling `.ps.graphics.create_device()` directly.

<img width="645" height="106" alt="Image" src="https://github.com/user-attachments/assets/f253896e-4a58-467c-ab07-cefbf37f22b6" />

There is no reasonable way for an end user to know to do this, and the fact that `ragg` is not working should not leave the user in a state where they can't plot at all. We should do one or more of the following:

- Test to see whether we can actually load `ragg` (in addition to checking its version) before enabling ragg graphics.
- If attempting to create the graphics device with `ragg` fails, fall back on the default (non-`ragg`) approach.
 

## @jmcphers at 2025-09-05T17:56:22Z

For anyone affected by this, you have basically 4 options:

1. install whatever libraries are necessary for `ragg` to work
2. remove the `ragg` package so ark doesn't try to use ragg
3. set `options(ark.ragg = FALSE)` in your `.Rprofile` so that ark doesn't try to use ragg
4. [run away from home and live in the woods](https://www.wikihow.com/Run-Away-from-Home-and-Live-in-the-Woods)

## @juliasilge at 2025-09-08T16:01:12Z

I believe we have yet another report of this: https://forum.posit.co/t/positron-experimental-graph-plot-editor-no-longer-available/206969

## @juliasilge at 2025-09-10T00:28:00Z

@DavisVaughan pointed out that likely we several reports of this all at once because ragg did a CRAN release and people didn't have versions of the package that were installed quite correctly. We'll want to make our plotting experience more robust to CRAN updates for ragg.

## @remlapmot at 2025-09-10T07:52:26Z

And in case it is helpful to know about ragg plots - likely in the future you'll get a Linux user confused why their plot with plotmath `expression()`s renders incorrectly, e.g.,

<img width="350" height="734" alt="Image" src="https://github.com/user-attachments/assets/c2c9e871-e3f7-43ff-aa30-057e7f0ac372" />

It turns out on Linux you need to change the symbol font from _Standard Symbols PS_ to a font which renders them correctly under ragg such as _Liberation Sans_ (and there are other fonts which render correctly), e.g.,

```r
systemfonts::register_variant("symbol", "Liberation Sans")
```

(For background see <https://github.com/r-lib/ragg/issues/136>, <https://github.com/r-lib/ragg/issues/89>, <https://github.com/r-lib/ragg/issues/201>, <https://github.com/r-lib/pkgdown/issues/2908>)