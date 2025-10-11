# Windows GHA workflow

> <https://github.com/posit-dev/ark/pull/175>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

- [x] Delete temporary 0.1.32 Release before merging

General idea is:
- `build_macos` to build the Mac `ark` binaries (arm64 and intel)
- `build_windows` to build the Windows `ark.exe` (only x64 for now, likely arm in the future?)
- `create_release` bundles the `modules/` into the Mac and Windows specific zip files

We get artifacts on the tags like:
- `ark-0.1.31-windows-x64.zip`
- `ark-0.1.31-debug-windows-x64.zip`

I've left in the `x64` here to go along with the `universal` of `darwin-universal` in our Mac builds. I assume in the future if we need ARM windows builds then we will either have a universal build that replaces this, or an additional `-arm64` artifact

