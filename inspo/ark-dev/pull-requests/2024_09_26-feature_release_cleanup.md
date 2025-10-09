# Clean up failed releases

> <https://github.com/posit-dev/ark/pull/546>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

It's currently possible for our workflows to leave a new release in a failed or incomplete state:

- A release build completes
- We create a new release
- But then we either fail to download release artifacts from build jobs, or fail to upload a release asset

To avoid this, we now clean up releases in case of propagated failure.

Another change made here is that the Slack failure report will now also be sent if an upload-release step fails. Previously we would only report build failures.

Making sure we don't create partial releases should help with https://github.com/posit-dev/positron/issues/4815.

Approach: This uses the `gh` CLI tool to check for existence of a release and delete it if any.

