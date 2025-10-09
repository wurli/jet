# Bump the IOPub "outbound high water mark" from 1k to 100k

> <https://github.com/posit-dev/ark/pull/129>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/1447

Tl;DR - zeromq is _dropping_ some of our IOPub (our pub) messages because the frontend (our sub) isn't able to process them fast enough. In particular it is dropping the all important IOPub _idle_ `'status'` message because we have clogged it with `'stream'` messages from the print loop.

A "simple" fix for the exact issue in the reprex is to raise the "high water mark".

FWIW this doesn't happen on the Python side because IPython seems to chunk the stdout into time buckets of `0.2` seconds, rather than emitting every line as its own `stream` message as R / ark does. We think the better solution is to follow this PR up with another that also does a similar thing on the ark side. We have had issues with the frontend Console being "jumpy" because of how expensive it is to process each individual line like this.

Note that upping this to 100k doesn't fix the problem altogether. Indeed, by going from 20k->200k iterations, I can reproduce it again:

```r
for (x in 1:200000) {
    print(sprintf("This qlwe qlejql ejkqw ejqw eqwej qwelqjwe qwejq wis row %d", x))
}
```

But we do think that 1k is much too low of a limit for such a destructive operation as dropping a message, so we plan to do _both_:
- Up the IOPub limit to 100k
- Batch the `stream` output we receive from R

---

See `ZMQ_SNDHWM` in http://api.zeromq.org/3-1:zmq-setsockopt, the "high water mark" for outbound messages:

> The ZMQ_SNDHWM option shall set the high water mark for outbound messages on the specified socket. The high water mark is a hard limit on the maximum number of outstanding messages ØMQ shall queue in memory for any single peer that the specified socket is communicating with. If this limit has been reached the socket shall enter an exceptional state and depending on the socket type, ØMQ shall take appropriate action such as blocking or dropping sent messages. Refer to the individual socket descriptions in [zmq_socket(3)](http://api.zeromq.org/3-1:zmq_socket) for details on the exact action taken for each socket type.

The default is 1000 for all message types.

If you look at http://api.zeromq.org/3-1:zmq-socket and the `ZMQ_PUB` section, you'll see that the high water mark behavior for Pubs is to _Drop_ messages over this limit.

---

I don't think there is an easy way to see how many messages are in the zmq pub queue at any time, which would have been a nice way to confirm this, but what I did instead was add some logging on the frontend to determine the time difference between when an IOPub `'stream'` message was created on the ark side and when it was actually received by the frontend. This is the code I ran:

```r
for (x in 1:20000) {
    print(sprintf("This qlwe qlejql ejkqw ejqw eqwej qwelqjwe qwejq wis row %d", x))
}
```

Note how as we near the 20000th iteration the frontend is getting more and more behind, providing strong evidence that the messages are getting backed up in the zmq queue.

```
# at the start, basically instant
created 2023-10-25T18:57:29.597204+00:00, received 2023-10-25T18:57:29.598Z 

# halfway through, ~3.8 seconds behind
created 2023-10-25T18:57:33.040487+00:00, received 2023-10-25T18:57:36.818Z 

# near the end, ~6 seconds behind
created 2023-10-25T18:57:35.142239+00:00, received 2023-10-25T18:57:41.333Z
```

## @softwarenerd at 2023-10-25T19:43:11Z

I was able to run 10's of thousands of stdout stream messages through.

![image](https://github.com/posit-dev/amalthea/assets/853239/4262a022-8a31-4172-8b60-9aa5b6e71b7a)

