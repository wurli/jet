# Support for browser and custom prompts

> <https://github.com/posit-dev/ark/pull/64>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Requires rstudio/positron#833
Addresses rstudio/positron#407
Progress towards rstudio/positron#827

- Detect browser prompt and treat them as top-level prompt instead of `input_request` prompts.

- Pass custom prompt information (including `Browser[n]` strings) via `ExecuteReply` and `LanguageInfo` messages (see rstudio/positron#833)


