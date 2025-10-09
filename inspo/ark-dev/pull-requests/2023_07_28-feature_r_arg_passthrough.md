# Make it possible to pass command line options to R

> <https://github.com/posit-dev/ark/pull/70>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This small change makes it possible to tell `ark` to pass arbitrary command line options to R when starting it. It uses the `--` convention; all of `ark`'s arguments are placed before the `--`, and any arguments that follow it are passed through, verbatim, to R when we call `Rf_initializeR`. 

The default behavior is to start R in `--interactive` mode as we did before. It's a bit of a judgement call whether we should also always pass `--interactive` when the user supplies their own R arguments. I chose not to since anyone using this probably knows what they're doing, but could see an argument for always including it.

Part of https://github.com/rstudio/positron/issues/930. 

## @kevinushey at 2023-07-28T16:20:06Z

LGTM!