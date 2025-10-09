# Add support for looking up dev topics on hover

> <https://github.com/posit-dev/ark/pull/233>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/2109

While this does address ^, it also has the nicer benefit of enabling us to, say, look up the dev docs of `vec_count()` when you hover over `vec_count()` when you have called `load_all()` while working on vctrs itself.

It does use an internal function from pkgload, `pkgload:::write_topic_html()`, but RStudio uses this too and I think it would be nice if pkgload itself officially exposed this. We do the exact same thing as RStudio, which is to copy out part of `print.dev_topic()` which writes the HTML to a file.
https://github.com/rstudio/rstudio/blob/4a20e65e129178ecadcffc53b7ad48b2c9781769/src/cpp/session/modules/SessionHelp.R#L781-L792
https://github.com/r-lib/pkgload/blob/7556a3f0a74e37afd5286b126b6b2321e563e761/R/dev-help.R#L86-L129

Problem from the original issue, fixed now:

<img width="956" alt="Screenshot 2024-02-02 at 12 28 40 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/c79edbeb-2524-4292-83d6-36ed6c8248a8">

New behavior of dev help on hover:


https://github.com/posit-dev/amalthea/assets/19150088/98ebba2b-daa1-4715-ae6e-4737ce4430ee




