---
description: Pick the next unblocked ready-for-agent issue (or a given #N), research the spec, plan, implement test-first in a worktree, verify on three axes, and open a PR for squash-merge.
argument-hint: "[#issue-number]"
allowed-tools: Task, Bash(gh:*), Bash(git:*), Read, Grep, Glob
---

You are the orchestrator for the forge issue-driven agent team. You run in the main session: you
dispatch subagents (Task tool), and you are the ONLY one who pauses at user gates. Follow this loop.

Argument: `$1` (optional issue number, may be empty).

## 0. SELECT
- If `$1` is given: read it (`gh issue view <n> --json number,title,body,labels`). Guard: it must be open and labeled `ready-for-agent`. Check it is unblocked (see graph below); if blocked, WARN, list blockers, and ask whether to continue.
- If `$1` is empty: build the worklist:
  1. `gh issue list --state open --label ready-for-agent --json number,title,body,labels`.
  2. For each, compute blockers from BOTH sources:
     - Native sub-issues: `gh api repos/{owner}/{repo}/issues/<n>/sub_issues` — an issue with open sub-issues is a parent/epic and is itself blocked.
     - Free-text: parse the `## Blocked by` section of the body for `#<n>`; "None"/empty = unblocked.
  3. Keep unblocked leaves (all blockers closed, no open sub-issues).
  4. Rank lowest issue number first. Print the ranked shortlist.
  5. Select the top one.

## 1. SPEC (plan-time)
Dispatch the `spec-oracle` subagent in plan mode with the issue number, title, and body. Capture its findings.

## 2. PLAN
Dispatch the `planner` subagent with the issue + the spec-oracle findings. It returns a plan doc path.

### ▒▒▒ GATE 1 ▒▒▒
Show the user the plan path and a summary. STOP and wait for explicit approval before any code.

## 3. SETUP
Create an isolated worktree for the issue (invoke `superpowers:using-git-worktrees`), branch named for the issue.

## 4. BUILD
Dispatch the `rust-implementer` subagent with the plan doc path + worktree path.

## 5. REVIEW (three axes, in parallel)
Dispatch in one batch:
- `reviewer` (standards) — worktree path + diff base.
- `spec-oracle` (review mode) — spec-oracle has no Bash; run `git -C <worktree> diff <base>` yourself and pass the diff TEXT in its prompt, along with the issue requirements.
- `qa` (behavior) — worktree path + the issue "Done when" criteria.

## 5b. INTEGRAL QA
The `qa` run includes the full-workspace `cargo test`. If any of the three verdicts is FAIL, feed the
findings back to `rust-implementer` and re-run review. Cap at 2 retry rounds; after that, surface the
failures to the user and stop. Always surface qa's ignored/skipped-test report.

## 6. PR
When all three verdicts PASS: push the branch and open a PR with `gh pr create`, body referencing
`Closes #<n>` and linking the plan doc.

### ▒▒▒ GATE 2 ▒▒▒
STOP. Do not merge. Tell the user the PR is ready for them to squash-merge. After they merge, the
issue auto-closes via `Closes #<n>`; offer to clean up the worktree.
