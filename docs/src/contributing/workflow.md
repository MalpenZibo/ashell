# Contribution Workflow

## Getting Started

1. **Fork** the repository on GitHub.
2. **Clone** your fork locally.
3. **Create a branch** from `main`:
   ```bash
   git checkout -b feat/my-feature
   ```
4. Make your changes.
5. **Run checks** before pushing:
   ```bash
   make check
   ```
6. **Push** and open a Pull Request against `main`.

## Branch Naming

Follow the conventional prefix pattern:

| Prefix | Purpose | Example |
|--------|---------|---------|
| `feat/` | New features | `feat/ddc-brightness` |
| `fix/` | Bug fixes | `fix/bluetooth-crash` |
| `chore/` | Maintenance, dependencies | `chore/update-iced-rev` |
| `docs/` | Documentation | `docs/developer-guide` |
| `refactor/` | Code restructuring | `refactor/network-service` |

## Maintainer Model

- **MalpenZibo** (Simone Camito) is the project creator and primary maintainer. Has final merge authority.
- **romanstingler** is a collaborator focusing on backend/service work, bug fixes, and Hyprland testing.
- **clotodex** is a collaborator providing Niri and NixOS testing and architectural feedback.

## Pull Request Process

1. PRs should target the `main` branch.
2. CI must pass (format, clippy, build).
3. At least one maintainer review is expected for non-trivial changes.
4. Keep PRs focused — one feature or fix per PR when possible.

## Issue Tracking

Issues are tracked on GitHub with labels:

- `bug` — Something is broken
- `enhancement` — Improvement to existing feature
- `feature` — New feature request
- `discussion` — Open-ended design discussion
- `good first issue` — Suitable for new contributors
- `help wanted` — Looking for community contributions
- `UI/UX` — User interface related
- `performance` — Performance improvements
- `blocked` / `postponed` — Cannot proceed currently

## AI-Assisted Contributions

AI-assisted contributions are accepted in this project. If you use AI tools to help write code, documentation, or other contributions, that is fine — the same quality standards apply regardless of how the code was written.

Using top-tier, frontier-class models is strongly recommended, but **you are responsible for the code you submit** no matter what tools you use. Review AI-generated output carefully, ensure it passes all checks (`make check`), and be prepared to explain and defend your changes during review.

For the full AI contribution guide including workflow recommendations, common pitfalls, and best practices, see [AI-Assisted Contributions](ai-assisted-contributions.md).

## Communication

- Primary communication is through GitHub issues and PR comments.
