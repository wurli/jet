# Handle when the graphics device doesn't provide a `holdflush()` callback

> <https://github.com/posit-dev/ark/pull/741>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

By mimicking `devholdflush()` in R itself, which just ignores the `level_delta` and uses a level of `0`.

Follow up to #732 

Our e2e tests test on Linux where the png backend is Cairo. In that case there is no wrapped `holdflush()` hook and we weren't handling that right. We need to ignore the `level_delta` that comes through and just return `0`. Previously we were returning `-1` and using that as the holdflush level because we expected the wrapped `holdflush()` hook to modify that for us. That caused us to permanently get stuck on `should_render = false`.

https://github.com/posit-dev/positron/actions/runs/13815181517/job/38656105150?pr=6748

