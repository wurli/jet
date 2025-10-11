# Disable proxies when connecting to local help server

> <https://github.com/posit-dev/ark/pull/928>
>
> * Author: @jmcphers
> * State: MERGED
> * Labels:

Ark's built-in help proxy doesn't work correctly when proxies are present, since the `reqwest` client, by default, uses the configured proxies. Of course, these proxies can't call back into localhost, so hilarity ensues.

Since we're always connecting to `localhost`, we don't ever want to use a proxy here; this change updates the reqwest client so that it doesn't use proxies.

Part of https://github.com/posit-dev/positron/issues/8426.

## @jmcphers at 2025-09-25T16:51:57Z

@juliasilge No, I don't think this will fix those problems. :-(
