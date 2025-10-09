# Update `.rs.api.versionInfo()`

> <https://github.com/posit-dev/ark/pull/531>
> 
> * Author: @juliasilge
> * State: MERGED
> * Labels: 

Addresses posit-dev/positron#4081
Addresses posit-dev/positron#4706
Goes together with posit-dev/positron#4703

There are a few differences between what I have in this draft and what `.rs.api.versionInfo()` returns for RStudio, especially on Workbench.

- ~~Currently `mode` will be "web" rather than "server" for non-desktop use. I did this because it is what VS Code uses internally throughout. Is it a problem for us to have something different here? See https://github.com/posit-dev/positron/pull/4703/files#r1764143168~~ We will have `mode` be "server", for consistency with RStudio.
- There is no `edition` or `release_name`, since Positron doesn't have those. The desktop version of RStudio also doesn't have `edition`.

## QA Notes

In the R console, you should see these types of results:

``` r
.rs.api.versionInfo()
#> $citation
#> To cite Positron in publications use:
#> 
#>   Posit team (2024). Positron: A next generation data science IDE.
#>   Posit Software, PBC, Boston, MA. URL https://www.posit.co/.
#> 
#> A BibTeX entry for LaTeX users is
#> 
#>   @Manual{,
#>     title = {Positron: A next generation data science IDE},
#>     author = {{Posit team}},
#>     organization = {Posit Software, PBC},
#>     address = {Boston, MA},
#>     year = {2024},
#>     url = {https://www.posit.co/},
#>   }
#> 
#> $mode
#> [1] "desktop"
#> 
#> $version
#> [1] '2024.9.0'
#> 
#> $long_version
#> [1] "2024.09.0+0"
#> 
#> $ark_version
#>                                              branch 
#>                                              "main" 
#>                                              commit 
#>                                          "8f5040d6" 
#>                                                date 
#>                           "2024-09-17 16:28:06 MDT" 
#>                                              flavor 
#>                                             "debug" 
#>                                                path 
#> "/Users/juliasilge/Work/posit/ark/target/debug/ark" 
#>                                             version 
#>                                           "0.1.136"
```

