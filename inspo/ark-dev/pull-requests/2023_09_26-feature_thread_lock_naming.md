# Print friendly name of thread for lock debugging

> <https://github.com/posit-dev/ark/pull/100>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This small tweak to the lock debugging code prints the friendly name of the thread that is asking for the lock, making it easier to see who's got it.

```
INFO crates/harp/src/lock.rs:51: Thread 'ark-environment' (ThreadId(25), nest level 0) is requesting R runtime lock.
```


## @DavisVaughan at 2023-09-26T21:05:37Z

```
---- object::tests::test_tryfrom_RObject_hashmap_string stdout ----
thread 'object::tests::test_tryfrom_RObject_hashmap_string' panicked at 'called `Result::unwrap()` on an `Err` value: InvalidUtf8(Utf8Error { valid_up_to: 10, error_len: Some(1) })', crates/harp/src/object.rs:743:64
```

im assuming this is resolved by the other PR

## @jmcphers at 2023-09-26T21:16:32Z

Yeah, it is!