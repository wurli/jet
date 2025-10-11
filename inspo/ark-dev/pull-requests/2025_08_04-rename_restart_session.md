# Update name of command for `rstudioapi::restartSession()`

> <https://github.com/posit-dev/ark/pull/891>
>
> * Author: @juliasilge
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/8794

This command got renamed during the multiple console session work. I took a quick look and I did not find anything else that needs to be renamed on a quick perusal.

### Release Notes

<!--
  Optionally, replace `N/A` with text to be included in the next release notes.
  The `N/A` bullets are ignored. If you refer to one or more Positron issues,
  these issues are used to collect information about the feature or bugfix, such
  as the relevant language pack as determined by Github labels of type `lang: `.
  The note will automatically be tagged with the language.

  These notes are typically filled by the Positron team. If you are an external
  contributor, you may ignore this section.
-->

#### New Features

- N/A

#### Bug Fixes

- Fixed the rstudioapi shim for `rstudioapi::restartSession()` (https://github.com/posit-dev/positron/issues/8794).


### QA Notes

In previous builds, running `rstudioapi::restartSession()` would error, but now it should restart R!

