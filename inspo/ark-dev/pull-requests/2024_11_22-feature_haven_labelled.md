# Add support for `haven_labelled` in the data explorer

> <https://github.com/posit-dev/ark/pull/634>
>
> * Author: @dfalbel
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/5327 by specializing some data explorer code paths in order to support the haven labelled data type.

At some point we might also want to implement an extension mechanism here too, allowing packages to implement a method that would create a proxy object - that's then used to compute the summary stats, histograms/freq tables, etc.

## @juliasilge at 2024-11-25T19:22:04Z

Does this also address posit-dev/positron#5010?

## @dfalbel at 2024-11-27T13:56:17Z

No, I think for the variables pane, we'll probably want to implement a custom `ark_display_value` method for haven_labelled.
With the variables pane, there's no way to decide wether we really want the quotes or not.
