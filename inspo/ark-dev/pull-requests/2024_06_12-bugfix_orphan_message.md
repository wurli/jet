# Serialize absent parents as empty dicts

> <https://github.com/posit-dev/ark/pull/400>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/2096

We found out with @DavisVaughan that the Jupyter protocol does allow orphan messages but the parent header must be an empty dict rather than null as is currently the case. This fixes jupyter-client based applications like jupyter-console which have been broken since we started to convert stdout/stderr to IOPub streams. Log messages emitted during startup ended up as malformed orphan messages.

