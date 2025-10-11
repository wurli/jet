# Drop reqwest's `default-features`

> <https://github.com/posit-dev/ark/pull/487>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Alternative to the OpenSSL part of https://github.com/posit-dev/ark/pull/486

The current version of reqwest has the following `default` features that we get automatically unless we opt out:
https://github.com/seanmonstar/reqwest/blob/193ed1fae73e9c44846a089506884d3b8f2865a6/Cargo.toml#L30

The `default-tls` feature automatically pulls in TLS related things that ultimately pull in openssl. But it seems like we don't actually need TLS features, so....let's not pull those in at all!

In the `Cargo.lock` now:

<img width="1021" alt="Screenshot 2024-08-28 at 1 42 43 PM" src="https://github.com/user-attachments/assets/1dcec674-f199-4c29-96ed-679a9be093e9">

Regarding the other reqwest default features that we are dropping now:
- charset: Might matter if we used `response.text()` but we only use `response.bytes()` as a passthrough intermediate https://github.com/seanmonstar/reqwest/blob/193ed1fae73e9c44846a089506884d3b8f2865a6/src/async_impl/response.rs#L161-L173
- http2: Enables HTTP/2 support, but I don't _think_ we need that?
- macos-system-configuration: Related to a system proxy created by `reqwest::proxy::system()`, which we don't use

