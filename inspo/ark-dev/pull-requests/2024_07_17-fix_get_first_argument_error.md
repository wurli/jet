# Log at info level for (mostly) expected try catch errors

> <https://github.com/posit-dev/ark/pull/440>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

There are a number of places where we try `r_parse_eval()` with the expectation that the user _may_ be writing pseudocode, and it's totally fine for the evaluation to fail with "object not found" errors.

Those cases were generating full backtraces and sometimes logging at error level, resulting in a lot of misleading noise when looking at the logs. Like this:

<img width="1048" alt="Screenshot 2024-07-13 at 6 05 54 PM" src="https://github.com/user-attachments/assets/13048767-d01a-4eb4-b0ba-2fe4bbc253ec">

It's not a bad idea to log something here, but I think we can just log the `message` and do it at info level, resulting in:

<img width="956" alt="Screenshot 2024-07-13 at 6 01 41 PM" src="https://github.com/user-attachments/assets/eea4f378-6196-40c8-bf0d-83f51700f2ee">


