# Remove invalid attempt to set `DYLD_FALLBACK_LIBRARY_PATH` and `LD_LIBRARY_PATH`

> <https://github.com/posit-dev/ark/pull/441>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Part of https://github.com/posit-dev/positron/issues/4048
Related to https://github.com/posit-dev/positron/pull/3921
Reverts a little bit of https://github.com/posit-dev/ark/pull/205, since we determined you must set these env vars in the parent process for them to have any affect.

On macOS you also need the `allow-dyld-environment-variables` entitlement so the child process can see it, which Positron currently sets for us.

