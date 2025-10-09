# Add option to allow unquoted formatting of strings

> <https://github.com/posit-dev/ark/pull/344>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

The objective of this is to address https://github.com/posit-dev/positron/issues/2865
Which is caused by `FormattedVector` automatically quoting strings when formatting string elements, which is used by the environments pane to display strings like this:

![image](https://github.com/posit-dev/amalthea/assets/4706822/8a24c479-b9f3-4706-9f31-0c24c38adbd4)

This PR adds an option to FormattedVector allowing to opt out of quoting.

I'm not sure we really want such option in FormattedVector, we could also skip using FormattedVector in the data explorer codebase instead, but maybe this can be useful in the future?

