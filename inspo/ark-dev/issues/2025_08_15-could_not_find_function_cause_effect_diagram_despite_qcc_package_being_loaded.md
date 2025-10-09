# Could not find function "causeEffectDiagram", despite qcc package being loaded

> <https://github.com/posit-dev/ark/issues/897>
> 
> * Author: @marioem
> * State: CLOSED
> * Labels: 

Trying to run `causeEffectDiagram` function in Positron console or from Quarto document in Positron, results in the following error:

```r
causeEffectDiagram(
  cause = list(Surroundings = cSurroundings,
               Policies = cPolicies,
               " " = NA,
               Systems = cSystems
  ),
  effect = cEffect)
title(adj = 1, cex.sub = 0.75)

Error in `causeEffectDiagram()`:
! could not find function "causeEffectDiagram"
```

Reprex (in Positron), however, works:

``` r
library(qcc)
#> Loading required package: ggplot2
#> Loading required package: patchwork
#> Package 'qcc' version 3.0
#> Type 'citation("qcc")' for citing this R package in publications.

cPolicies <- c("Price change", "No regular maintenance done", "Screen change", "Staff not available for maintenance activities", "Lack of maintenance", 
               "Insufficient workforce", "Low staff budget", "Lack of TPM", "Licence not issued for show")
cSurroundings <- c("Show timings changed", "Low occupancy", "Social reasons", "Show cancelation")
cSystems <- c("AC not working", "Recliner seats not reclining", "Damaged seats", "Online connectivity issue while ticket booking", "Ticket denial", "Double booking")

cEffect <- "Viewers are not happy\nwith their experience"

causeEffectDiagram(
  cause = list(Surroundings = cSurroundings,
               Policies = cPolicies,
               " " = NA,
               Systems = cSystems
  ),
  effect = cEffect)
```

![](https://i.imgur.com/T1Q6dwZ.png)<!-- -->

``` r
title(adj = 1, cex.sub = 0.75)
#> Error in title(adj = 1, cex.sub = 0.75): plot.new has not been called yet

sessionInfo()
#> R version 4.5.1 (2025-06-13)
#> Platform: x86_64-apple-darwin20
#> Running under: macOS Sequoia 15.6
#> 
#> Matrix products: default
#> BLAS:   /Library/Frameworks/R.framework/Versions/4.5-x86_64/Resources/lib/libRblas.0.dylib 
#> LAPACK: /Library/Frameworks/R.framework/Versions/4.5-x86_64/Resources/lib/libRlapack.dylib;  LAPACK version 3.12.1
#> 
#> locale:
#> [1] en_US.UTF-8/en_US.UTF-8/en_US.UTF-8/C/en_US.UTF-8/en_US.UTF-8
#> 
#> time zone: UTC
#> tzcode source: internal
#> 
#> attached base packages:
#> [1] stats     graphics  grDevices utils     datasets  methods   base     
#> 
#> other attached packages:
#> [1] qcc_3.0         patchwork_1.3.1 ggplot2_3.5.2  
#> 
#> loaded via a namespace (and not attached):
#>  [1] crayon_1.5.3       vctrs_0.6.5        cli_3.6.5          knitr_1.50        
#>  [5] rlang_1.1.6        xfun_0.52          generics_0.1.4     labeling_0.4.3    
#>  [9] glue_1.8.0         htmltools_0.5.8.1  scales_1.4.0       rmarkdown_2.29    
#> [13] grid_4.5.1         evaluate_1.0.4     tibble_3.3.0       MASS_7.3-65       
#> [17] fastmap_1.2.0      yaml_2.3.10        lifecycle_1.0.4    compiler_4.5.1    
#> [21] dplyr_1.1.4        fs_1.6.6           RColorBrewer_1.1-3 pkgconfig_2.0.3   
#> [25] farver_2.1.2       digest_0.6.37      R6_2.6.1           tidyselect_1.2.1  
#> [29] reprex_2.1.1       pillar_1.11.0      magrittr_2.0.3     tools_4.5.1       
#> [33] withr_3.0.2        gtable_0.3.6
```

<sup>Created on 2025-08-15 with [reprex v2.1.1](https://reprex.tidyverse.org)</sup>

This code works in RStudio.

I assume it is more ark-related than Positron, as ark handles code execution.

Positron Version: 2025.08.0 build 130
Code - OSS Version: 1.102.0
Commit: 76ddce53e85437b013671fe7d91a3a1c54f48341
Date: 2025-08-01T20:09:11.051Z (1 wk ago)
Electron: 35.6.0
Chromium: 134.0.6998.205
Node.js: 22.15.1
V8: 13.4.114.21-electron.0
OS: Darwin x64 24.6.0

ark version: 0.1.201 (#888)

R version: 4.5.1

BRs,

Mariusz

## @DavisVaughan at 2025-08-15T14:54:51Z

Check the R version that Positron is running (top right corner of Positron).

It looks like qcc added `causeEffectDiagram()` in the dev release but that isn't on CRAN yet.

https://github.com/luca-scr/qcc/blob/c1c90199fdb02fd6582d8c7493a8f9ff666ff2d6/R/deprecated.R#L29-L33

So you may have dev qcc installed in the version of R that reprex uses (the command line `R`), but release qcc installed in the version of R that Positron is choosing to use by default

## @marioem at 2025-08-15T15:04:16Z

Hi,

<img width="139" height="71" alt="Image" src="https://github.com/user-attachments/assets/60bb6055-4bf8-43a4-9f7e-0561d94b6eb3" />

I have single R version and packages environment. Same qqc lib for both RStudio and Positron, works in RStudio, reprex, but not in Positron with ark.

BRs,

Mariusz

## @DavisVaughan at 2025-08-15T15:06:47Z

I think you definitely have two versions of qcc installed.

I see `qcc_3.0` in your reprex output.

But CRAN qcc is only at 2.7 https://cran.r-project.org/web/packages/qcc/index.html

Try `packageVersion("qcc")` in Positron. If that returns 2.7 then you definitely have two different versions of qcc installed.

## @DavisVaughan at 2025-08-15T15:07:23Z

Can you also please run `.libPaths()` in both Positron and with reprex and paste your output?

## @marioem at 2025-08-15T15:51:00Z

Hmm, does running `.libPaths()` has some side effects in ark?

From Positron console:

```r
> .libPaths()
[1] "/Library/Frameworks/R.framework/Versions/4.5-x86_64/Resources/library"
```

But now loading qcc is more verbose than it used to be (before it was silent).

```r
> library(qcc)
Loading required package: ggplot2
Loading required package: patchwork
  __ _  ___ ___ 
 / _  |/ __/ __|  Quality Control Charts and 
| (_| | (_| (__   Statistical Process Control
 \__  |\___\___|
    |_|           version 3.0
Type 'citation("qcc")' for citing this R package in publications.
```

And the function is now found.

```sh
 (base)  mariusz@mamini  /Library/Frameworks/R.framework/Versions  17:38  ls
3.5        3.6        4.0        4.1        4.2        4.3-x86_64 4.4-x86_64 4.5-x86_64 Current
 (base)  mariusz@mamini  /Library/Frameworks/R.framework/Versions  17:38  find . -type d -name "qcc"         
./4.5-x86_64/Resources/library/qcc
./4.2/Resources/library/qcc
./4.4-x86_64/Resources/library/qcc
 (base)  mariusz@mamini  /Library/Frameworks/R.framework/Versions  17:40  more ./4.5-x86_64/Resources/library/qcc/DESCRIPTION  | grep Version 
Version: 3.0
 (base)  mariusz@mamini  /Library/Frameworks/R.framework/Versions  17:41  more ./4.4-x86_64/Resources/library/qcc/DESCRIPTION  | grep Version
Version: 3.0
 (base)  mariusz@mamini  /Library/Frameworks/R.framework/Versions  17:42  more ./4.2/Resources/library/qcc/DESCRIPTION  | grep Version
Version: 2.7
```

It must have been some issue with loading the package before executing `.libPaths()`, as there is only R 4.5.1 available to Positron:

<img width="715" height="706" alt="Image" src="https://github.com/user-attachments/assets/0eb745a5-f0b2-4052-b2c3-5d19ca0fcd04" />

Can't find other explanation.

BRs,

Mariusz

## @DavisVaughan at 2025-08-15T16:13:01Z

I don't think running libPaths had any affect. I think you just restarted R and now it is working. You likely had some weird broken R state before. 