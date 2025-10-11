# Avoid warning with invalid regex filter in Data Explorer

> <https://github.com/posit-dev/ark/pull/477>
>
> * Author: @dfalbel
> * State: MERGED
> * Labels:

Addresses: https://github.com/posit-dev/positron/issues/4392
The error was caused because `grepl` in R will show a warning and an error when an invalid regex appears.
We already catch the error, but the warning was leaking to the console.

QA: Following the issue description should not show any console warning any longer.

