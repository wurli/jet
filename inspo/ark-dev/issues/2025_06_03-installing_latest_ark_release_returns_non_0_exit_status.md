# Installing latest ark release returns non-0 exit status

> <https://github.com/posit-dev/ark/issues/820>
> 
> * Author: @JosiahParry
> * State: CLOSED
> * Labels: 

At work we're building a Docker image to integrate with our jupyter instances. We're trying to use Ark for the kernel. 

However, when using the latest release of ark, `ark --install` returns the error `Error: A connection file must be specified. Use the `--connection_file` argument.` 

It looks like this PR from ~4 months ago fixed it: https://github.com/posit-dev/ark/pull/712.

In the Dockerfile we have as a workaround: 

```dockerfile
RUN ./ark --install || true
````

However, this will suppress any actual errors. We will try using the latest pre-release version tomorrow and see if that addresses the issue. Though we try not to use unofficial software releases whenever possible.

Would it be possible to push another "official" in the near future? 

Thank youu!!!

## @DavisVaughan at 2025-06-03T12:58:05Z

We don't have releases more "official" than what you see in https://github.com/posit-dev/ark/tags, those are our "releases" that we use directly in Positron, so you should expect them to be fairly stable (i.e. they shouldn't just crash to to logic errors on our part, since we use it in Positron), but they absolutely may contain breaking changes

## @JosiahParry at 2025-06-03T13:00:24Z

I understand, yes. Each release since December 2024 has been marked as a “pre-release”. Is the intention to only use pre-release tags?

## @DavisVaughan at 2025-06-03T13:16:13Z

Odd! I don't see any change between these two releases that would have changed it from `latest` -> `pre-release`
https://github.com/posit-dev/ark/compare/0.1.159...0.1.160

We do set `prerelease: true` here, but we've been doing that for years
https://github.com/posit-dev/ark/blob/15ea580f8ae32a78ab299650378369e67cae9e6c/.github/workflows/release.yml#L95

In October we switched from `actions/create-release@v1` to `softprops/action-gh-release@v2`, but we did releases between October -> December that show up as true releases, so it doesn't seem like it was that transition that changed anything.

## @DavisVaughan at 2025-06-03T13:18:10Z

Oh, actually, it's the other way around, only this 1 release is marked as a non-pre-release
https://github.com/posit-dev/ark/releases/tag/0.1.159

All other releases seem to be pre-releases, which seems right to me.

## @DavisVaughan at 2025-06-03T13:20:24Z

I don't know why 0.1.159 was marked as release, I think that was a mistake. I have manually changed it to pre-release to now match all other releases. I think just using the latest pre-release is what you'll want.

## @JosiahParry at 2025-06-03T14:00:05Z

Okay, nice! If they're _all_ marked pre-release thats very helpful. Thank you!!