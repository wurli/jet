# `tibble::view()` fails in Ark (& Positron) with "variable names are limited to 10000 bytes" error

> <https://github.com/posit-dev/ark/issues/864>
> 
> * Author: @coatless
> * State: CLOSED
> * Labels: 

### Description

The `tibble::view()` function (accessed via `%&gt;% view`) fails when executed in Ark and Positron environments, while working correctly in RStudio and R.app. This prevents users from viewing data frames using the tidyverse pipe syntax.

### Reproduction Steps

```r
library(tidyverse)
library(nycflights13)
df1 <- weather %>% view
```

### Expected vs Actual Behavior

| ✅ **Working Environments**<br>(RStudio, R.app) | ❌ **Failing Environments**<br>(Ark, Positron) |
|:---:|:---:|
| **R.app (macOS)** | **Positron** |
| <img width="395" alt="Screenshot of R.app showing a spreadsheet-style data viewer displaying weather data with columns for origin, year, month, day, hour, temperature, dewp, humidity, wind direction, wind speed, wind gust, and precipitation." src="https://github.com/user-attachments/assets/3985c32f-a432-4187-a321-fecd859fae0c" /> | <img width="395" alt="Screenshot of Positron IDE showing tibble::view() error with 'Error in exists(): variable names are limited to 10000 bytes' message and failed df1 assignment." src="https://github.com/user-attachments/assets/9e6e1fb9-4904-4339-8c5c-27e2036400e3" /> |
| **RStudio** | **Ark (Jupyter Console)** |
| <img width="395" alt="Screenshot showing RStudio's data viewer successfully displaying the weather dataset with filter options and successful command execution." src="https://github.com/user-attachments/assets/dec5ee91-97dd-4343-93f0-e64563054d6e" /> | <img width="395" alt="Screenshot of Jupyter console showing Ark R session with tibble::view() error and 'variable names are limited to 10000 bytes' error message." src="https://github.com/user-attachments/assets/c9f57b18-d078-471c-a839-89fd25c9f399" /> |

### Error Details

**Error Message:**
```r
Error in `exists()`:
! variable names are limited to 10000 bytes
```

**Stack Trace:**
```
Hide Traceback
    ▆
 1. ├─weather %>% view
 2. ├─tibble::view(.)
 3. │ └─rlang::eval_tidy(quo(view_fun(!!x, !!title)))
 4. └─view_fun(`<tibble[,15]>`, ".")
 5.   ├─base::isTRUE(exists(name, envir = env, inherits = FALSE))
 6.   └─base::exists(name, envir = env, inherits = FALSE)
```

### Possible cause

The issue appears to stem from how `tibble::view()` attempts to retrieve the `View` function:

```r
view_fun <- get("View", envir = as.environment("package:utils"))
```

**Reference:** [ark/src/modules/positron/view.R#L51](https://github.com/posit-dev/ark/blob/74d3f1da6ce2aca47b54153dde456028fb6e06dd/crates/ark/src/modules/positron/view.R#L51)

https://github.com/posit-dev/ark/blob/74d3f1da6ce2aca47b54153dde456028fb6e06dd/crates/ark/src/modules/positron/view.R#L51

The `view_fun()` function is attempting to retrieve the `View` function from the utils package, but this lookup mechanism seems to be incompatible with Ark's environment setup, resulting in the "variable names are limited to 10000 bytes" error during the `exists()` call.

### Environment Information

- **Tidyverse version:** 2.0.0
- **R version:** 4.5.1
- **Affected platforms:** Ark 0.1.195, Positron 2025.07.0 (Universal) build 204
- **Working platforms:** RStudio (Version 2025.05.1+513 (2025.05.1+513)), R.app (R 4.5.1 GUI 1.82 High Sierra build (8536))

Reported initially on [Reddit](https://www.reddit.com/r/rstats/comments/1lqhy4k/comment/n1627uj/?utm_source=share&utm_medium=web3x&utm_name=web3xcss&utm_term=1&utm_content=share_button) by [atthemost7](https://www.reddit.com/user/atthemost7/).

## @juliasilge at 2025-07-04T14:25:26Z

This works for me with tibble v3.3.0:

<img width="1312" height="912" alt="Image" src="https://github.com/user-attachments/assets/3ef28591-9276-48df-8873-99170ee32944" />

Is it possible that this is the same as https://github.com/posit-dev/positron/issues/4702 and https://github.com/posit-dev/positron/issues/5392 and the version of tibble is not up to date?

## @coatless at 2025-07-04T16:39:41Z

@juliasilge yes exact issue. Sorry for the noise.

tibble 3.2.1 (installed) vs. tibble 3.3.0 (patched)

## @juliasilge at 2025-07-04T19:18:08Z

Great! Glad to hear it 🙌 