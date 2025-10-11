# Substitute LF for CRLF in code to be parsed or executed

> <https://github.com/posit-dev/ark/pull/112>
>
> * Author: @jennybc
> * State: MERGED
> * Labels:

To address https://github.com/posit-dev/positron/issues/1520

Here's how the problem presents (this is "before"). We have problems both with detecting a complete statement and also with execution.

https://github.com/posit-dev/amalthea/assets/599454/e1ab6edd-3aed-49d7-a8cb-97a76196b754

Here's behaviour with the fix (this is "after"). We are able to detect statement completeness and we can execute code.

https://github.com/posit-dev/amalthea/assets/599454/e072030c-7683-41d9-b085-0b09b704432d

