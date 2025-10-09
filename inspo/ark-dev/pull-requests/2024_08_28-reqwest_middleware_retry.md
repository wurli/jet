# Add retries to Help proxy request

> <https://github.com/posit-dev/ark/pull/489>
> 
> * Author: @juliasilge
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/3753

Folks in that issue ⤴️ are experiencing problems seeing R Help pages in the Help pane. Some things we can notice:

- None of us internally have been able to reproduce the problem.
- Some folks _do_ occasionally see the Help page they are querying for! Seems like an intermittent problem.
- We see errors in logs like "operation was canceled: connection closed before message completed".
- Some folks have their problems totally solved after installing a new version of Positron.

Given all that, seems like we may just need to retry the request, or at the least that is a first possible step as outlined by @DavisVaughan in https://github.com/posit-dev/positron/issues/3753#issuecomment-2245984690.

