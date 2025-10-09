# Fix row labels of data explorer

> <https://github.com/posit-dev/ark/pull/247>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

- Addresses https://github.com/posit-dev/positron/issues/2273
- Fixes performance with large data
- Also we now check bounds of request parameters and fail if the request is for long vectors. Note that neither base R nor tidyverse support data frames containing long vectors.

The main fix is that we now subset the data before doing anything else. This prevents materialising row names for the whole dataset and makes sure the row labels are for the relevant subset of data.

Before:

https://github.com/posit-dev/amalthea/assets/4465050/6a9a17ee-17c9-43c8-a00d-fd3390aeb23c

After:

https://github.com/posit-dev/amalthea/assets/4465050/caa6aeec-dd94-4b95-818b-b664c34ce577


