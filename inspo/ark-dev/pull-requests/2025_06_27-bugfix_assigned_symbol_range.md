# Extend range of assignment symbols to end of node

> <https://github.com/posit-dev/ark/pull/860>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Branched from #858

This fix is similar to https://github.com/posit-dev/ark/pull/855

It makes sure the range of document symbol of assigned objects fully extends up to the end of the RHS. This allows the breadcrumbs to behave much better with the method symbols implemented in #858.

Before:

https://github.com/user-attachments/assets/737ab393-89c2-4471-a08a-1d50d7e78982

After:

https://github.com/user-attachments/assets/50d41990-2019-4f9b-9e32-8e699f31d6fe



