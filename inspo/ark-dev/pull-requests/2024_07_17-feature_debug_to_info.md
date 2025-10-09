# Remove usage of `debug!` in favor of `info!` or `trace!`

> <https://github.com/posit-dev/ark/pull/442>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/4023

Most of the uses of `debug!` have been moved to `info!` because I think they provide valuable information that will be useful when users report issues. A few have been moved to `trace!` instead, if they really felt to be very low level

Along the way, modernized to use `log::error!()` instead of imported `error!()` in the files I touched

