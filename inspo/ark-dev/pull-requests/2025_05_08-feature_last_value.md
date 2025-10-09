# Add option to show `.Last.value` in the Variables pane

> <https://github.com/posit-dev/ark/pull/794>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

Quick airplane feature to optionally show `.Last.value` in Positron's Variables pane when the option `positron.show_last_value` is set.

Note that the option can be turned on during a session, but turning it off requires a restart. It's likely most users who want this will set it in their `.Rprofile`; we could also add a Positron setting for this.

Partially addresses https://github.com/posit-dev/positron/issues/3034.

