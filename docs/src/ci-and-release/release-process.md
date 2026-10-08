# Release Process

ashell uses [cargo-dist](https://github.com/axodotdev/cargo-dist) (v0.30.0) for automated release builds and GitHub Releases.

> **Note:** Releases are managed by the project maintainer (@MalpenZibo). This page documents the process for reference.

## How a Release Works

1. **Draft release notes**: The `release-drafter.yml` workflow automatically drafts release notes based on merged PRs. The maintainer reviews and edits the draft in GitHub Releases.

2. **Publish the release**: The maintainer publishes the drafted release (e.g., tag `v0.11.0`). This triggers `pre-release.yml`, which:
   - Prepends the release notes to `CHANGELOG.md`
   - Sets the Cargo version with `cargo set-version`
   - Freezes the website docs version (`docusaurus docs:version`)
   - Commits and pushes to `main`, moves the tag to the new commit
   - Triggers the Release workflow (`release.yml`) with the tag

   The Release workflow can also be run manually via Actions → Release → Run workflow.

3. **Automated pipeline**: The release workflow:
   - Runs `dist plan` to determine build matrix
   - Builds platform-specific artifacts (Linux binary + archives)
   - Builds global artifacts (shell installer)
   - Generates .deb and .rpm packages via `generate-installers.yml`, then installs each one in clean `ubuntu:24.04`, `debian:trixie` and `fedora:latest` containers and runs `ashell --version`. A package that fails to install blocks the release.
   - Uploads all artifacts to the GitHub Release
   - Un-drafts the release

4. **Post-release**: Downstream packaging jobs run automatically:
   - `update-arch-package.yml` (currently a placeholder that only prints a message)
   - `remove-manifest-assets.yml` cleans up dist manifests from the release

## cargo-dist Configuration

`dist-workspace.toml` configures the release build:

```toml
[workspace]
members = ["cargo:."]

[dist]
cargo-dist-version = "0.30.0"
ci = "github"
installers = ["shell"]
targets = ["x86_64-unknown-linux-gnu"]
```

## Dry Run

To test the release process without actually publishing, the maintainer can:

1. Go to Actions → Release → Run workflow
2. Enter `dry-run` as the tag
3. This runs the full pipeline but doesn't create a GitHub Release

## Versioning

- Version is defined in `Cargo.toml`: `version = "0.11.0"`
- Tags follow semver: `v0.11.0`
- Pre-releases use suffixes: `v0.12.0-beta.1`
- The `--version` flag shows: `ashell 0.11.0 (abc1234)` (version + git hash)
