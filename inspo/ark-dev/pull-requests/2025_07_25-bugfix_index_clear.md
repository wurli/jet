# Clear index on `did_close`

> <https://github.com/posit-dev/ark/pull/881>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/8668

### QA Notes

Renaming files should not leave dangling workspace symbols (cmd/ctrl + t):


https://github.com/user-attachments/assets/1ede9524-5558-41a0-9567-622a74f4db06



## @lionel- at 2025-07-25T20:06:46Z

Wait that's not right, it's not about opened files it's about workspace files, so we'll need to manually watch for file deletion.

## @DavisVaughan at 2025-07-25T21:55:40Z

Surely there's some LSP notification we can get about file renames?
