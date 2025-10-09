# Add package information in completions

> <https://github.com/posit-dev/ark/pull/616>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/5225

### Positron Release Notes

#### New Features

- Completions for R functions and objects now display which package they are from (posit-dev/positron#5225). And completions for package namespaces are now indicated by a `::` suffix to make it clearer what is being completed.


#### Bug Fixes

- N/A

## @DavisVaughan at 2024-10-31T12:32:54Z

I think we also need to let the client know we support this
https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_completion

Look for `labelDetailsSupport` there.

It's possible we could be working with a client that _doesn't_ support this, but it is >3 years old now so its probably ok to ignore the client supplied option for now (we should still set the server side one though)

## @lionel- at 2024-11-04T12:35:09Z

Another change I've made is to add `::` as label details for namespace completions.

Before:

<img width="530" alt="Screenshot 2024-11-04 at 13 28 00" src="https://github.com/user-attachments/assets/4b4fe16a-2838-47a9-a42e-26e69d797d8e">

After:

<img width="536" alt="Screenshot 2024-11-04 at 13 29 40" src="https://github.com/user-attachments/assets/a86ace13-a273-4358-80f5-ef4fab4bcb06">

The goal is to make it clearer that a namespace is being completed and `::` will be inserted.