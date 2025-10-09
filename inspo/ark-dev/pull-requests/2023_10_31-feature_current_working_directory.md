# Deliver working directory changes to Positron

> <https://github.com/posit-dev/ark/pull/133>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change is a companion to https://github.com/posit-dev/positron/pull/1740; it causes ark to emit events to Positron when the working directory changes. The events are emitted over the frontend comm, and are always fed to the comm when it is first created so that Positron has a view of the initial working directory.

This change doesn't depend on https://github.com/posit-dev/positron/pull/1740, nor does https://github.com/posit-dev/positron/pull/1740 depend on it; they can be merged independently.

