# Modernize all GHA dependencies

> <https://github.com/posit-dev/ark/pull/600>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/4821

Here is the current 0.1.145

<img width="1384" alt="Screenshot 2024-10-21 at 4 51 31 PM" src="https://github.com/user-attachments/assets/38c32d93-a2b2-4dcb-b3eb-dab1ec320a49">

Here is a manual trigger of 0.1.146 with this PR at https://github.com/posit-dev/ark/actions/runs/11448395058. Note, no warnings at the bottom of this runs page!


<img width="1415" alt="Screenshot 2024-10-21 at 5 00 36 PM" src="https://github.com/user-attachments/assets/4af8e8db-179a-4b31-af29-ad66f27a4171">



## @DavisVaughan at 2024-10-22T12:46:30Z

They were deprecated in 2021 without replacement https://github.com/actions/upload-release-asset, https://github.com/actions/create-release

That official deprecated repos recommend this one, and it by far has the most love and activity
https://github.com/actions/upload-release-asset?tab=readme-ov-file#github-action---releases-api

It is also what Positron has been using for some time now, so I think it is okay