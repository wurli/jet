# Fix `r_task::initialize()` timing issue

> <https://github.com/posit-dev/ark/pull/566>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Both `modules::initialize()` and `resource_loaded_namespaces()` call `r_task::spawn_idle()`. Even though that is async, it blocks until the task channels are initialized, so `r_task::initialize()` has to run first!

---

We didn't catch this during testing because `r_task::spawn_idle()` has that escape hatch during testing and just runs immediately. But CI in Positron caught an issue, and it was easy to reproduce by just starting up R with the latest version of ark in Positron. Things crashed immediately.

## @lionel- at 2024-10-05T05:49:04Z

> We didn't catch this during testing because r_task::spawn_idle() has that escape hatch during testing and just runs immediately.

That's a good reason to switch to an independent binary in integration tests, so we don't hit any testing path at all: #562.