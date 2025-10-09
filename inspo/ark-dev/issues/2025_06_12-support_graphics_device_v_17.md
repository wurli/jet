# Support graphics device v17

> <https://github.com/posit-dev/ark/issues/838>
> 
> * Author: @DavisVaughan
> * State: CLOSED
> * Labels: 

We'll need to add `GEDevDescVersion17`, `DevDescVersion17`, and `pDevDescVersion17` and pay close attention to what's changed between 16 and 17

And then add a 17 branch to this
https://github.com/posit-dev/ark/blob/3d42829c911c27f2ace0565378cd881e434ac8c6/crates/ark/src/plots/graphics_device.rs#L722-L725

If I'm understanding this commit correctly, then nothing actually changed between 16 and 17 in the DevDesc and GEDevDesc objects themselves, so we can probably just copy/paste the existing code and bump the number:
https://github.com/wch/r-source/commit/a98d9f88c7f73e5f4da8209d9373bf4316ee2d10

