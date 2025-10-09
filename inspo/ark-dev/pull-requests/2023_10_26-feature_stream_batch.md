# Buffer stdout/stderr `'stream'` messages

> <https://github.com/posit-dev/ark/pull/131>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 



## @DavisVaughan at 2023-10-27T21:30:25Z

@lionel- I've implemented most of your suggestions. They really simplified things!

I've also switched from flushing on every non `Stream` message to flushing _right before_ we send out a few key messages. I tried to exactly match what ipykernel does, i.e.:
- Right before display_data, and update_display_data https://github.com/ipython/ipykernel/blob/2a8adb921a562e5d24376b2de40f7e31e02f50f8/ipykernel/zmqshell.py#L102
- Right before going idle after a control or shell message https://github.com/ipython/ipykernel/blob/2a8adb921a562e5d24376b2de40f7e31e02f50f8/ipykernel/kernelbase.py#L347, https://github.com/ipython/ipykernel/blob/2a8adb921a562e5d24376b2de40f7e31e02f50f8/ipykernel/kernelbase.py#L432
- Right before an `execute_reply` of some kind (an actual reply or an error reply) https://github.com/ipython/ipykernel/blob/2a8adb921a562e5d24376b2de40f7e31e02f50f8/ipykernel/kernelbase.py#L761

They also flush right before an `input_request`
https://github.com/ipython/ipykernel/blob/2a8adb921a562e5d24376b2de40f7e31e02f50f8/ipykernel/kernelbase.py#L1260

I don't think we really should have to do this, but we definitely have to do something because of https://github.com/posit-dev/positron/issues/1700. Right now I am having us try to:
- Wait until all messages in the IOPub queue have cleared
- ...plus wait another 200ms

It isn't enough to just wait for the IOPub queue to clear, because that only means that we sent the messages to the frontend, not that the frontend actually processed them from that socket yet. So in theory we could send an IOPub message over to the frontend, but if it is backed up then it won't process it immediately. Since the IOPub queue looks cleared that frees our "wait" block but then an input request could be sent and immediately processed by the frontend stdin socket before that IOPub message we sent over is processed. This sounds hypothetical but it happened a lot when fiddling with these test cases:

```r
for (x in 1:20000) {
    print(sprintf("This qlwe qlejql ejkqw ejqw eqwej qwelqjwe qwejq wis row %d", x))
    if (x == 10000) {
        readline("hi there>")
    }
}

for (x in 1:20000) {
    print(sprintf("This qlwe qlejql ejkqw ejqw eqwej qwelqjwe qwejq wis row %d", x))
    if (x == 10000) {
        plot(1:100)
    }
}

for (x in 1:20000) {
    print(sprintf("This qlwe qlejql ejkqw ejqw eqwej qwelqjwe qwejq wis row %d", x))
    if (x == 10000) {
        plot(1:100)
    }
    if (x == 15000) {
        abline(1, 2)
    }
}
```

I really don't think it is on ark to solve this. We are taking extra effort to ensure that the IOPub messages are _sent_ before the `input_request` is, but that still isn't enough. I think ideally the IOPub messages could arrive after the `input_request` with no issues, and the frontend would just wait until the `input_request` is done to actually process the IOPub messages, as outlined in https://github.com/posit-dev/positron/issues/1700, then we could remove the "wait" and sleep bits but keep everything else.

## @DavisVaughan at 2023-11-02T13:40:18Z

I actually found a jupyter notebook issue that talks about this issue too https://github.com/jupyter/notebook/issues/3159