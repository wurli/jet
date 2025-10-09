# Fix package name extraction from help path

> <https://github.com/posit-dev/ark/pull/537>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/3775

The TLDR of https://github.com/posit-dev/positron/issues/3775 is that normally libpaths look like this:

```r
> .libPaths()
[1] "/Users/davis/Library/R/arm64/4.4/library"                            
[2] "/Library/Frameworks/R.framework/Versions/4.4-arm64/Resources/library"
```

And help paths look like this

```r
> unclass(help("match"))
[1] "/Library/Frameworks/R.framework/Versions/4.4-arm64/Resources/library/base/help/match"
```

So we would look for `/library/<package>` to extract the package name.

But it turns out that the `.libPaths()` prefix part of the help path can be anything! Here's what renv uses:

```r
> .libPaths()
[1] "/Users/davis/Desktop/myRProject/renv/library/macos/R-4.4/aarch64-apple-darwin20"                       
[2] "/Users/davis/Library/Caches/org.R-project.R/R/renv/sandbox/macos/R-4.4/aarch64-apple-darwin20/f7156815"
```

```r
> unclass(help("glue_data"))
[1] "/Users/davis/Desktop/myRProject/renv/library/macos/R-4.4/aarch64-apple-darwin20/glue/help/glue"
```

See the `/library/macos` there? `macos` isn't a package! So our package name resolution falls over here. We end up passing `package = "macos"` to `tools::Rd2HTML()` and it eventually triggers a warning of `no package 'macos' was found`.

---

I experimented with a few ways to better get at that package name, and ultimately I think the most reliable way is to recognize that `utils:::.getHelpFile()` has to resolve the package name too to figure out which Rd database to pull the Rd file from. It uses the fact that help paths always look like this:

```
<libpath>/<package>/help/<topic>
```

Where:
- `<libpath>/<package>/help` is a true folder on the file system, and the Rd database lives in that folder
- `<libpath>/<package>/help/<topic>` doesn't actually exist, but gives you the topic name. The path structure is just a convention.

So `.getHelpFile()` just does `basename(dirname(dirname(path)))` to get at the package name. I think we can keep it simple and do exactly this, trusting that base R won't change something this critical to the help system.

## @DavisVaughan at 2024-09-18T19:28:38Z

I had a bit of trouble getting renv to install a package _without_ first installing it into the cache (i.e. I needed glue to be in the libpath that has `/library/macos` in it).

Ultimately this worked

```r
# Prevent renv from installing glue in the cache
options(renv.config.cache.enabled = FALSE)

renv::install("glue")

library(glue)

# It should show `library/macos` in the path if
# the cache is actually disabled
unclass(help("glue_data"))

# Hover over this, then run something in the console
# to trigger the warning
glue_data()
```

Here is the problem in action:


https://github.com/user-attachments/assets/eb08d486-60ca-4cdb-af38-d7cc415450ad

And of course, now that it is fixed notice it says `{glue}` as the package up there in the top left of the help box

<img width="618" alt="Screenshot 2024-09-18 at 3 34 10 PM" src="https://github.com/user-attachments/assets/2be9bdf0-563d-4bbb-925a-c7ea3e56d85c">

