# Revert "Async callbacks for processing GetColumnProfile RPC requests"

> <https://github.com/posit-dev/ark/pull/513>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Reverts posit-dev/ark#473

After merging #473, I bumped the Ark version and tried that version in https://github.com/posit-dev/positron/pull/4326 (CI was failing before because it required the R changes). Turns out that removing the R failures uncovered some issues with the Python implementation that only happen on CI. 

Since we can't land  https://github.com/posit-dev/positron/pull/4326 right now, we'll need to revert #473, otherwise we can't bump Ark for Positron if we need to include other changes. If we bump Ark in Positron with the changes in #473 without the changes from https://github.com/posit-dev/positron/pull/4326, then the Data Explorer will stop working.

@lionel- Do you have better ideas how to avoid this situation?
I think ideally we would need to be able to build ark releases from branches, and then being able to specify them for Positron PR's, so we can test the full solution on CI before merging into Ark.





## @lionel- at 2024-09-10T07:34:24Z

> I think ideally we would need to be able to build ark releases from branches, and then being able to specify them for Positron PR's, so we can test the full solution on CI before merging into Ark.

@dfalbel Good question, I was wondering the same in another context!

I think ideally we'd support specifying a github branch of ark in a file somewhere in the positron repo, and then CI would detect it and build ark from there (very few lines of shell are needed to set up and launch an ark build) and use the resulting binary instead of a release build. This setup would be less heavy than having to do ark releases just for testing.