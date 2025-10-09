# remove the `dummy.do_something` command

> <https://github.com/posit-dev/ark/pull/110>
> 
> * Author: @seeM
> * State: MERGED
> * Labels: 

Following https://github.com/posit-dev/positron/pull/1476, we will start a new ARK language client for each notebook. This currently tries to re-register a global command with ID `dummy.do_something` which fails once more than one language client have initialized.

## @seeM at 2023-10-11T15:27:25Z

Oops made this too son, will bump the version etc now.

EDIT: Oh I misunderstood how that worked. We can bump and build a release after this is merged (when we're ready).

## @jmcphers at 2023-10-11T19:10:48Z

Am going to go ahead and merge this and and bump the version so @seeM can pick it up in his PR. 