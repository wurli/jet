# Avoid crashing when given input longer than R's buffer

> <https://github.com/posit-dev/ark/pull/202>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/1767
Which is this TODO left for ark https://github.com/posit-dev/positron/issues/1326#issuecomment-1745389921

Ideally the frontend will break long inputs up for us, making this redundant, but until then we need to at least avoid crashing.

I've taken the same approach as RStudio, which is to just trim the input if it exceeds the buffer size. This isn't ever a practical issue in RStudio because it seems to split large chunks of input by newlines, and sends one line at a time to R's buffer.

Our previous behavior of just returning without copying anything into the buffer wasn't working, and would result in a crash, presumably because R was expecting _something_ to get copied into the buffer after a read-console iteration.


