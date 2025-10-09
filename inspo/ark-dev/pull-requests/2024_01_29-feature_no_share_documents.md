# Remove global `DOCUMENT_INDEX`

> <https://github.com/posit-dev/ark/pull/220>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Should unblock https://github.com/posit-dev/amalthea/pull/218

Our workspace "indexer" was adding every file in the workspace into the `documents` DashMap used by the LSP. This was in addition to the LSP backend trying to track `did_open()` and `did_close()` events using the same `document` map. The indexer didn't actually use the `documents` map for anything, so all this was doing was making it difficult for the LSP to track "open" documents.

## @kevinushey at 2024-01-29T19:36:25Z

To provide a bit more context, IIRC the intention here was for the indexer to help out with diagnostics; e.g. if you run diagnostics on the whole project, it would be useful if we could reuse a document index already constructed for files existing but not open in a project.

Does that workflow still have a way forward?

## @DavisVaughan at 2024-01-29T20:04:57Z

I _think_ so because I think we'll be able to check for that file's existence in the `WORKSPACE_INDEX` in addition to this `documents` dash map somewhere along the way and somehow merge the two when doing project wide diagnostics

## @kevinushey at 2024-01-29T21:22:50Z

Aha, perfect!