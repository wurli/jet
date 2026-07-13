# Releasing jet

1. Bump `[workspace.package]` version in `Cargo.toml` on a branch.
2. Open PR, merge to `main`.
3. Pull the merged commit, then tag and push:
   ```bash
   git checkout main && git pull
   git tag v0.2.0
   git push origin v0.2.0
   ```
4. The tag push triggers the workflow, which builds macOS (arm64) and
   Linux (x86_64, arm64) binaries and publishes a GitHub Release with:
   - `jet-<target>.tar.gz` (CLI) + `jet-installer.sh`
   - `jet_lua-<target>.tar.gz` (Neovim cdylib)

   Release notes are auto-generated. Edit them after publishing if
   desired — the artifacts and download URLs are unaffected.

## If the build fails

Fix forward — open another PR, merge, delete the bad tag and release,
then re-tag:

```bash
gh release delete v0.2.0 --yes
git tag -d v0.2.0 && git push --delete origin v0.2.0
git tag v0.2.0 && git push origin v0.2.0
```

## Configuration

Release builds are driven by [cargo-dist](https://opensource.axo.dev/cargo-dist/).
Config lives in `dist-workspace.toml` and per-package `[package.metadata.dist]`
blocks; the workflow itself (`.github/workflows/release.yml`) is generated
by `dist generate` and shouldn't be edited by hand.
