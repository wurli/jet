# Ark: Sign binaries in macOS releases

> <https://github.com/posit-dev/ark/issues/698>
>
> * Author: @lionel-
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABSPDwBw", name = "bug", description = "Something isn't working", color = "E99695")

The Ark binaries in https://github.com/posit-dev/amalthea/releases are currently not signed. This makes it hard to download and use ark in Jupyter apps.

## @DavisVaughan at 2024-06-21T15:52:54Z

Currently you manually approve

<img width="983" alt="Screenshot 2024-06-21 at 11 02 17 AM" src="https://github.com/posit-dev/positron/assets/19150088/23894ae2-a177-407b-b7e2-f5f3e495b7f4">


## @DavisVaughan at 2024-06-26T19:36:33Z

Turns out that you cannot staple a notarization ticket to a _binary_, which makes it pretty much impossible to improve our current approach of downloading a binary version of ark directly. Jupyter users will always get that message about ark not being verified.
https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution/customizing_the_notarization_workflow#3087720

I studied rig a bit, and I think we can do what rig does, but the Makefile for it is a little cryptic:
https://github.com/r-lib/rig/blob/main/Makefile

IIUC, the general idea is:
- `codesign` ark, with hardened runtime and entitlements
- `pkgbuild` ark into a _component_, where the component has an internal file structure of `/usr/local/bin/ark`
- `productbuild` to make a `.pkg` containing that 1 ark component, and additional `--resources` like our NOTICE and LICENSE files
- `xcrun notarytool` to notarize the `.pkg` and staple the notarization ticket to the `.pkg` (which is allowed)
- Distribute the `.pkg` with its stapled ticket

The user side then looks like:
- Download the `.pkg` and open it. Should not get any warnings because we signed and notarized it.
- That runs the installer, the user basically just clicks through it and hits Install (this shows them the license document too)
- It installs ark into `/usr/local/bin/ark`

Then at the command line they can immediately run `ark --install` without ark needing special treatment to be on the PATH, because of its placement in `/usr/local/bin`. And because it came from the `.pkg` it should be blessed as well, and not get quarantined by Gatekeeper.

We could also probably auto run `ark --install` for them? So if they go through the installer then all they'd need to do is open Jupyter. But that may be too much.

## @jmcphers at 2024-06-26T21:26:15Z

> Jupyter users will always get that message about ark not being verified.

As I read it we can notarize but not staple -- which means that the ticket is still _there_, Gatekeeper just needs to validate it online instead of checking a local copy. You wouldn't get the message unless you're offline. Does that sound right?

## @DavisVaughan at 2024-06-26T21:36:41Z

I tried exactly that - i.e. this actions release actually succeeded because i removed the staple step https://github.com/posit-dev/ark/actions/runs/9684264850

But when I downloaded ark I still got the error about it not being able to identify the owner 😢 it is possible I still have something wrong though

(I have since deleted that ark release with its artifact but we can retry anytime)

## @DavisVaughan at 2024-06-26T21:41:31Z

In particular if I double click on the `ark` executable I get this

<img width="250" alt="Screenshot 2024-06-26 at 5 40 24 PM" src="https://github.com/posit-dev/positron/assets/19150088/84d5f0d8-54ce-489b-b193-1253b346f6bf">

If I try and run it from the command line I get this

<img width="253" alt="Screenshot 2024-06-26 at 5 40 39 PM" src="https://github.com/posit-dev/positron/assets/19150088/a96119a7-f7e1-413d-804a-f597aff6af8b">

