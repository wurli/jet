# Use Positron Bot creds for CLA assistant

> <https://github.com/posit-dev/ark/pull/455>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change replaces the `CLA_ASSISTANT_PAT` secret (which is known to occasionally 403) with a workflow that uses an ephemeral PAT issued to the Positron Github app via Positron Bot.

This is the same workflow we use in the main Positron repository.

