# AI-Assisted Contributions

## Project Stance

AI-assisted contributions are accepted in ashell. If you use AI tools to help write code, documentation, or other contributions, that is fine — the same quality standards apply regardless of how the code was written.

That said, **generating code is the easy part**. The critical challenge is understanding what the AI produced and ensuring it fits the project. All code review is done manually by maintainers in their free time, and review remains the bottleneck — not implementation.

**The bottom line: you are responsible for the code you submit, no matter how it was written.**

## Guidelines

### Prefer Frontier-Class Models

Using top-tier, frontier-class models (e.g., Claude Opus or equivalent) is strongly recommended. Lower-capability models tend to produce subtly incorrect code, miss architectural conventions, or introduce patterns that don't fit the codebase. You can use whatever tool you prefer, but the quality bar for contributions does not change — if the output doesn't meet it, you'll be asked to revise.

### You Own Your Code

Regardless of the tools used, **you are responsible for the code you submit**. Review AI-generated output carefully, ensure it passes all checks (`make check`), and be prepared to explain and defend your changes during review.

### Discuss Before Implementing

Before working on a feature or large change, **talk to the maintainers first**. Open an issue or comment on an existing one to discuss:

- Does this fit the project?
- What architectural decisions make sense?
- Are there constraints or context that aren't obvious from the code?

This applies to all contributions, AI-assisted or not, but is especially important when AI makes it cheap to generate large amounts of code that may not be wanted.

### Small, Incremental PRs

Big refactors and complex changes take a long time to review manually. Keep PRs focused and incremental. One feature or fix per PR. This is better for everyone: easier to review, easier to revert if needed, and less risk of regressions hiding in large diffs.

## Workflow Recommendations

### Research → Plan → Implement (RPI)

For non-trivial work, an effective workflow is:

1. **Research** — understand the codebase, existing patterns, and constraints
2. **Plan** — design the approach, discuss it with maintainers
3. **Implement** — execute the plan

This avoids the common failure mode of generating plausible-looking code that doesn't fit the project's architecture or conventions.

### Self-Review Before Requesting Review

Make your code review-ready before asking maintainers to look at it. This means:

- All checks pass (`make check`)
- You've read through the diff yourself
- The PR description explains what changed and why
- You've verified the change works as intended

Don't loop maintainers into reviews before you consider the code "good". This is a hobby project maintained in people's free time — respect that time.

### Focus AI Narrowly on Review Feedback

After receiving review comments, **constrain your AI tool to only address the specific feedback**. A common failure mode is pointing AI at review comments and having it "fix" them while also making unsolicited changes elsewhere — destroying code that was already reviewed and approved. Be explicit: only modify what was commented on.

## Where AI Works Well

- **Documentation and grammar** — especially for non-native English speakers
- **Quick prototyping** — exploring how a feature might look or work
- **Refactoring discovery** — finding code that could be improved (but verify suggestions match project intent — some patterns are intentional)
- **Boilerplate and repetitive patterns** — when the pattern is well-established in the codebase

## What to Watch Out For

- **Hallucinations** — LLMs can fabricate plausible-looking but incorrect code. Always verify the output against the actual codebase.
- **Added complexity** — AI tends to over-engineer: adding unnecessary abstractions, caching layers, or pre-loading logic that makes things worse, not better. Start simple.
- **Shifting bugs around** — fixing an issue in one place while introducing the same issue elsewhere. Review the full scope of changes, not just the area you asked it to fix.
- **Overgeneration** — LLMs produce verbose code by default. Prefer minimal, lightweight solutions and only add complexity when needed.
- **Outdated knowledge** — AI suggestions may be based on outdated training data. Verify that patterns, APIs, and conventions match the current state of the codebase.

## Review Process

- All code review is manual, performed by maintainers in their free time
- Documentation PRs may receive lighter review since they don't affect runtime behavior
- Complex or architectural changes will take longer to review — this is expected
- The project prefers getting it right over getting it fast
