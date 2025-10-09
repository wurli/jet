# Embed modules in binary

> <https://github.com/posit-dev/ark/pull/223>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

With this PR we now use rust-embed to embed the module files in the binary. This will make ark easier to package across platforms as we no longer need to make any assumptions/indirections about where to find our local resources. In debug builds we still watch the module files for change and source from there if any, we're still in a good position to debug.

The change is mostly straightforward. We parse the files from a text string created from the embedded memory and source the resulting expressions with `source(exprs = exprs)`. Most things work as before, including the detection of exported objects using srcrefs (they are still created even when parsing from a string).

I struggled for a while trying to source directly from the static memory, but turns out this is not possible as rust-embed hides the static refs behind a Cow pointer that immediately becomes owned (i.e. no longer points to the original location but to its own allocated memory) for some reason.

Adds a bunch of helpers to source / parse from different data sources.

## @lionel- at 2024-01-30T17:44:43Z

🤔 I don't think the Harp modules worked in release builds at all! `harp/src/modules` was never bundled in these releases. Should now be fixed as they are embedded in the binary as of the last commit.

## @DavisVaughan at 2024-01-31T14:57:32Z

> I don't think the Harp modules worked in release builds at all

@lionel- yea that's what i think i was messaging you on slack about a week or so ago haha

## @DavisVaughan at 2024-01-31T14:58:18Z

I think you should do a companion Positron PR that removes the code that copies around the modules folders, if you haven't already

## @DavisVaughan at 2024-01-31T17:36:37Z

Noting that I think this can be removed with this PR
https://github.com/posit-dev/positron/blob/5bec3c4cae8d725962b5a4842b742bb9b743cc9f/extensions/positron-r/scripts/install-kernel.ts#L264