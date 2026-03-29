# Release Process

ashell uses [cargo-dist](https://github.com/axodotdev/cargo-dist) (v0.30.0) for automated release builds and GitHub Releases.

## How to Create a Release

1. **Draft release notes**: The `release-drafter.yml` workflow automatically drafts release notes based on merged PRs. Review and edit the draft in GitHub Releases.

2. **Trigger the release**: Go to Actions → Release → Run workflow. Enter the version tag (e.g., `v0.8.0`).

3. **Automated pipeline**: The release workflow:
   - Runs `dist plan` to determine build matrix
   - Builds platform-specific artifacts (Linux binary + archives)
   - Builds global artifacts (shell installer, checksums)
   - Generates .deb and .rpm packages via `generate-installers.yml`
   - Uploads all artifacts to the GitHub Release
   - Un-drafts the release

4. **Post-release**: Downstream packaging jobs run automatically:
   - `update-arch-package.yml` updates the AUR package
   - `copr-build.yml` builds the Fedora COPR package
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

To test the release process without actually publishing:

1. Go to Actions → Release → Run workflow
2. Enter `dry-run` as the tag
3. This runs the full pipeline but doesn't create a GitHub Release

## Versioning

- Version is defined in `Cargo.toml`: `version = "0.7.0"`
- Tags follow semver: `v0.7.0`
- Pre-releases use suffixes: `v0.8.0-beta.1`
- The `--version` flag shows: `ashell 0.7.0 (abc1234)` (version + git hash)
