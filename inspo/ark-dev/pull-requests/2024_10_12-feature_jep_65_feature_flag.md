# Move protocol version to Amalthea and add feature flag

> <https://github.com/posit-dev/ark/pull/584>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Closes #578.

- Amalthea now sets the protocol version. Bumped from 5.3 to 5.4 (which states that shutdown requests on Shell are deprecated, we didn't support those anyway).

- Added feature flag for JEP 65 (#577) in `kernel_info_reply` where we now set `support_iopub_welcome` to `true`. Kallichore will be able to check for this flag to determine whether to use the improved kernel readiness detection, cc @jmcphers.

  Edit: We now use JEP 92 instead which stipulates a `supported_features` field, see message in thread.

  Approach: There is a new variant of `kernel_info_reply` with a `_full` suffix that contains the private fields. The Amalthea users (e.g. Ark) continue to use the former and Amalthea uses the full version internally. Ideally it'd be a `pub(crate)` but this causes consistency issues e.g. with `Message` being `pub`.

## @lionel- at 2024-10-12T16:48:25Z

Now uses JEP 92 (https://github.com/jupyter/enhancement-proposals/blob/master/92-jupyter-optional-features/jupyter-optional-features.md) to advertise the feature flag.

Here is how it's implemented in ipykernel: https://github.com/ipython/ipykernel/blob/3089438d84447bf859556b89f528ef59332803c5/ipykernel/kernelbase.py#L898

The JEP states that supported features can be:

> An optional feature can be a list of additional messages and/or a list of additional fields in different existing messages.

JEP 65 implements a new message of type `"iopub_welcome"` so that's what our new `supported_features` field now contains.