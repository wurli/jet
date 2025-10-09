# Add other formats for plot save

> <https://github.com/posit-dev/ark/pull/311>
> 
> * Author: @timtmok
> * State: MERGED
> * Labels: 

Address https://github.com/posit-dev/positron/issues/2657

UI PR: https://github.com/posit-dev/positron/pull/2729

During my testing, I had trouble saving SVG until I installed XQuartz. I'm not sure if R users will find this to be a problem on Mac. It seems that it's not common to use `grDevices::svg()` to save as SVG.

