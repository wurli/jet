# Eliminate the snippet source

> <https://github.com/posit-dev/ark/pull/782>
> 
> * Author: @jennybc
> * State: MERGED
> * Labels: 

Closes #779 
Closes #780
Addresses https://github.com/posit-dev/positron/issues/3108

Relates to https://github.com/posit-dev/positron/issues/7234. Morally, it doesn't quite close that issue, because we might want to do some things on the Positron side, such as (1) add R and/or Python examples to the placeholder content of newly created snippet files (now in https://github.com/posit-dev/positron/pull/7401) and (2) add documentation about R snippets in Positron vis-a-vis how it works in RStudio (now in https://github.com/posit-dev/positron-website/pull/67).



## @jennybc at 2025-04-28T17:37:48Z

@DavisVaughan Yeah I have convinced myself that it makes sense to represent something like `for` as a naked keyword and also in snippet form. Happy also to level up the `repeat` treatment. Until this PR it wasn't completed at all, so there was no *status quo* to preserve. But all your comments agree with my inclinations re: rationalizing the treatment of reserved words.

## @DavisVaughan at 2025-04-28T18:33:29Z

Great, let's try representing all `KEYWORD_SNIPPETS` as both bare completions and snippet completions. If you need any more thoughts on how to resolve the completion map key duplication issue, just ping me!