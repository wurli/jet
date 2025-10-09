# Support for displaying field data types in the connections pane

> <https://github.com/posit-dev/ark/pull/208>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Adresses https://github.com/posit-dev/positron/issues/2043
Requires front-end changes in https://github.com/posit-dev/positron/pull/2091

This PR adds support for displaying field data types in the connections pane. The connections pane requires implementors to return a data.frame containing columns `name` and `type` that we then forward to the front end to be displayed in the UI like in the image below:

![image](https://github.com/posit-dev/amalthea/assets/4706822/341a6dbf-347f-490e-aea7-11ba8f3d08a9)


