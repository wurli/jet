# Export `.rs.api.getVersion()` and `.rs.api.getMode()`

> <https://github.com/posit-dev/ark/pull/605>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/5094

`rstudioapi::getMode()` is new as of rstudioapi 0.17.0

For `rstudioapi::getVersion()`, this function did exist but it used to be this:

```
> rstudioapi::getVersion
function () 
{
    verifyAvailable()
    callFun("versionInfo")$version
}
```

Note the `getVersion()` function calls `callFun("versionInfo")` to indirectly get at `$version`.

We do provide `.rs.api.versionInfo()`, so this was working before.

Now it more directly calls:

```
> rstudioapi::getVersion
function () 
{
    if (hasFun("getVersion")) 
        return(callFun("getVersion"))
    verifyAvailable()
    base <- .BaseNamespaceEnv
    version <- base$.Call("rs_rstudioVersion", PACKAGE = "(embedding)")
    package_version(version)
}
```

So we need to provide the more direct accessor of `callFun("getVersion")` / `.rs.api.getVersion()`, which this PR does.

---

I have tested that `getMode()` and `getVersion()` work with CRAN rstudioapi, and that `getVersion()` continues to work with older versions of rstudioapi too.

## @DavisVaughan at 2024-10-23T11:48:59Z

Oooh I wonder if I should try and write an integration test for this! Would probably need to get the rstudioapi package installed on CI but that's easy these days

## @lionel- at 2024-10-23T12:16:53Z

@DavisVaughan Agreed, I was also thinking we should now test the rstudioapi integration.

## @DavisVaughan at 2024-10-23T14:46:47Z

Includes some nice little stdout when the test is skipped

<img width="695" alt="Screenshot 2024-10-23 at 10 43 43 AM" src="https://github.com/user-attachments/assets/0f938cee-d66d-4eda-842b-ddd27bd2e190">
