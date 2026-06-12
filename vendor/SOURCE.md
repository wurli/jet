# Vendored kallichore OpenAPI spec

`kallichore.json` is fetched verbatim from the upstream
[posit-dev/kallichore](https://github.com/posit-dev/kallichore) repo and
consumed by `build.rs` (via `progenitor`) to generate the Rust client
in `src/kallichore/api.rs`.

## Source

- Upstream: <https://github.com/posit-dev/kallichore/blob/main/kallichore.json>
- Pinned commit: `9ca5338d52c5299451193fe446563873c810aa17` (2026-06-10)
- License: Elastic License 2.0 (the spec only — generated client is our code)

## Updating

```sh
# fetch HEAD and update the SHA above
curl -sSL https://raw.githubusercontent.com/posit-dev/kallichore/main/kallichore.json \
  -o vendor/kallichore.json
# rebuild — progenitor runs as a build script
cargo build
```

Pin to a specific commit instead of `main` when you want a reproducible
update; `git ls-remote https://github.com/posit-dev/kallichore main` gives
the SHA.
