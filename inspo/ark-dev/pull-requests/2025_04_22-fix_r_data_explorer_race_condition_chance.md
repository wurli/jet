# Register our console prompt listener before sending `CommManagerEvent::Opened`

> <https://github.com/posit-dev/ark/pull/783>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Closes #781 - see my comment there for full analysis

To try and avoid any potential race conditions here where a critical prompt update may come in right after we send out `Opened` but before we've registered a listener, resulting in us missing the prompt update.

I'm going to optimistically say this closes that issue, and we can revisit if it pops up again.

This feels like it would be a super rare race condition, but there are atomics involved, so who knows.

