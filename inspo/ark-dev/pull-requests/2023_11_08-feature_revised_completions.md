# Split completion work into multiple files

> <https://github.com/posit-dev/ark/pull/141>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

I'm planning some larger completion related revisions, but I'm having a lot of trouble navigating our single `completions.rs` file that contains everything. Completions are complex enough that I think they should have their own folder of files, so I've done a _pure refactor_ PR that doesn't change any behavior, but does split the completions into the following parts:
- Types, containing a few common types
- Completion items, containing constructors for a `CompletionItem` from various input types
- "Resolving" completions (i.e. like providing help), this is a separate LSP command from just providing completions.
- 3 files that append actual completions, based on their location
  - Session
  - Workspace
  - Document

I don't think this will be super controversial and I'd like to keep some momentum going here, so I'm going to go ahead and merge 

