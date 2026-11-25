---
name: prepare-pr
description: Get an ashell change ready for review and open its pull request - scope and issue check, docs, a simplify pass and a correctness review, make check, a live test, then the pull request. Use it whenever you are about to open a pull request on ashell (gh pr create or any other way), or when asked to submit or prepare a change for review.
---

# Prepare a pull request

Follow these steps before opening any pull request on ashell, whoever asks for it. Talk to the
user in the language they write in; write code, commits and the pull request in English. The
rules of the project are in `AGENTS.md` and `docs/src/contributing/`: follow them throughout.

These steps are the default, not a gate: if the user asks to skip one, or to open the pull
request right away, do so and mention in your report what was skipped.

## 1. Scope

- The base is `main` of MalpenZibo/ashell: `origin/main`, or `upstream/main` when `origin` is
  the contributor's fork (add the `upstream` remote if it is missing). Below, `origin/main`
  means that base.
- Fetch it, then read `git log --oneline origin/main..HEAD` and `git diff origin/main...HEAD`:
  that is what the pull request will contain.
- Work on a branch named as "Branch naming" in `AGENTS.md` says; if you are on `main`, create
  one first. If it conflicts with `origin/main`, rebase it.
- A feature or a large change needs an issue where a maintainer agreed with it
  (`gh issue view <number> --comments`). If there is none, tell the user that the maintainers
  ask to discuss features first, and go on only once they confirm that a maintainer agreed,
  that they are a maintainer, or that they want to skip this check.
- One feature or fix per pull request. Anything that slipped in and isn't needed for it (a
  second fix, a refactor of code the change doesn't need, formatting of untouched lines) goes
  to a branch of its own: ask the user before moving it.

## 2. Docs

If the change touches behaviour, configuration, a command or the architecture, update in the
same pull request the docs that describe it, as "Documentation" and "Keep docs in sync" in
`AGENTS.md` say. For a new or changed translation, follow "Adding a Translation" in
`docs/src/contributing/common-tasks.md`.

## 3. Simplify, then review

Review the whole diff against `origin/main` in two separate passes, simplification first, then
correctness: a pass that looks for everything at once misses more.

- In Claude Code, run `/simplify`, then `/code-review`.
- With other agents, do each pass with its list below, or use the agent's own review command
  for the correctness pass (Codex CLI has `/review`). If the agent can start a subagent or a new
  session, run each pass there with only the diff and the list, not your own reasoning about
  the change: an agent that rereads what it just wrote tends to agree with it.

Simplification:

- something ashell already has (a helper, a component, a service) that the change writes again;
- code that can go: unused parameters, branches that can't be reached, copies of the same logic;
- abstractions, caches, options or config keys nobody asked for;
- names that don't say what a thing is;
- comments that repeat what the code says.

Correctness:

- bugs and cases not handled: empty or missing values, a service that isn't available, a
  compositor or backend without the feature;
- regressions in what used to work, and configs that used to load and no longer do;
- the rules of `AGENTS.md`: docs in sync, translations, no iced in `crates/ashell-services`.

Act on what matters: bugs, regressions, broken rules, code that will mislead the next reader.
Leave out style preferences and speculative refactors. Problems that were there before the
change go in your report to the user, not into this branch. If the fixes were substantial, run
both passes once more, then stop.

For a change that touches only docs (`website/`, `docs/`, Markdown files), proofread them
instead of running the two code passes.

## 4. Checks

Run `make check` and fix until it is clean. For a change under `experimental/`, also run that
workspace's checks (see `experimental/README.md`). A change that touches only docs can skip
`make check`.

## 5. Try it

If the change affects what ashell does at runtime, build it and run it in a real session
(`make build`, then `make start`). Running it replaces the user's bar, so ask before stopping the one that is
running. If you can't (no Wayland session, or the change targets a compositor or hardware you
don't have), ask the user to try it and note in the pull request that you couldn't.

For a visible change, tell the user that screenshots or a short recording on the pull request
would be welcome.

## 6. Commit

Commits follow "Commit conventions" in `AGENTS.md`, with a body that says why. One commit, or a
few that each make sense alone.

## 7. Open

- `git push -u origin <branch>` (the contributor's fork, if they work on one).
- `gh pr create --base main` with the title in the commit format. In the body: what changed
  and why, how it was checked (commands, compositor, what was tried), what was left out, and
  `Closes #<number>` when it closes an issue. Keep it short. If the user wants to write the
  description themselves, open the pull request with an empty body.
- `gh pr checks --watch`. If CI fails, fix what failed, run that check again, then commit the
  fix and push it. Review the fix only if it changes behaviour.
- Never merge the pull request, and never push to `main`.
