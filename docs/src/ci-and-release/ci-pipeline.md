# CI Pipeline

ashell uses GitHub Actions for continuous integration. All workflow files are in `.github/workflows/`.

## Main CI Workflow (`ci.yml`)

**Trigger**: Push to `main`, pull requests targeting `main`.

**Runner**: `ubuntu-24.04`

**Steps**:

1. **Install dependencies**: All system libraries needed for compilation
   ```bash
   sudo apt-get install -y pkg-config llvm-dev libclang-dev clang \
     libxkbcommon-dev libwayland-dev dbus libpipewire-0.3-dev \
     libpulse-dev libudev-dev
   ```

2. **Format check**: `cargo fmt --all -- --check`
   - Fails if any code is not properly formatted.

3. **Clippy lint**: `cargo clippy --all-features -- -D warnings`
   - Zero warnings policy. All clippy warnings are treated as errors.

4. **Build**: `cargo build`
   - Ensures the project compiles successfully.

## Nix CI (`nix-ci.yml`)

Verifies that the Nix flake builds correctly.

## Website CI

- **`gh-pages-test.yml`**: Tests the Docusaurus website build on PRs.
- **`gh-pages-deploy.yml`**: Deploys the website to GitHub Pages on push to main.

## Dependency Management (`dependabot.yml`)

Dependabot is configured to:
- Check for Rust dependency updates (Cargo)
- Check for GitHub Actions updates
- Create PRs for available updates

## All Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `ci.yml` | Push/PR to main | Format, lint, build |
| `nix-ci.yml` | Push/PR | Nix flake validation |
| `release.yml` | Manual dispatch | Build release artifacts |
| `pre-release.yml` | Pre-release tag | Pre-release builds |
| `generate-installers.yml` | Called by release | Build .deb/.rpm packages |
| `gh-pages-deploy.yml` | Push to main | Deploy website |
| `gh-pages-test.yml` | PR | Test website build |
| `copr-build.yml` | Release | Fedora COPR package |
| `update-arch-package.yml` | Release | Update AUR package |
| `release-drafter.yml` | Push/PR | Auto-draft release notes |
| `remove-manifest-assets.yml` | Post-release | Clean up dist manifests |
