# Issue-Driven Agent Team Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a `/work-next-issue [#N]` orchestrator command plus five specialist subagents that pick the next unblocked `ready-for-agent` GitHub issue, research the KerML spec, plan, implement test-first in a worktree, verify on three axes, and open a PR for the user to squash-merge.

**Architecture:** The orchestrator is a slash command running in the main session (it alone can dispatch subagents and pause at user gates). It delegates to five isolated subagents in `.claude/agents/`. The `spec-oracle` is the sole holder of the spec MCP tools and is consulted twice (plan-time and diff-time). Each subagent invokes the relevant superpowers skill so engineering discipline is consistent.

**Tech Stack:** Claude Code project commands (`.claude/commands/*.md`) and subagents (`.claude/agents/*.md`); `gh` + `git` CLIs; the `spec` MCP server (`mcp__spec__*`); superpowers skills.

## Global Constraints

- Deliverables are committed on branch `agent-team-issue-driven` (already created), never on `main`.
- Subagents cannot dispatch other subagents and cannot pause for user input — only the command (main session) does orchestration and gates.
- Every subagent whose `tools` are restricted MUST include `Skill` (to invoke superpowers skills) and the tools it actually needs — nothing more (least privilege).
- `spec-oracle` is the ONLY agent granted `mcp__spec__*` tools; it has no `Edit`/`Write`/`Bash`.
- Model: omit the `model` frontmatter key on every agent → inherits the session model.
- MCP tool names are referenced by full identifier, e.g. `mcp__spec__search_sections`.
- Plans/specs live in `docs/plans/` (repo convention), not `docs/superpowers/`.
- Agent file frontmatter keys: `name`, `description`, optional `tools` (comma-separated). Command frontmatter keys: `description`, `argument-hint`, optional `allowed-tools`.
- Design source of truth: `docs/plans/2026-06-25-issue-driven-agent-team-design.md`.

## Shared Subagent Contracts

The orchestrator and each subagent communicate by prompt-in / final-message-out. Names below are the `subagent_type` / `name` values; later tasks depend on these exact strings.

| Subagent | Input (orchestrator provides in prompt) | Output (final message) |
|---|---|---|
| `spec-oracle` (plan mode) | issue number + title + body | Spec findings: clause IDs, BNF rule names, figure refs, bulleted requirements |
| `spec-oracle` (review mode) | worktree path + diff base + issue requirements | Verdict `SPEC: PASS|FAIL` + violation list with clause refs |
| `planner` | issue + spec findings | Absolute path to the written plan doc in `docs/plans/` |
| `rust-implementer` | plan doc path + worktree path | Summary of commits + `cargo test` status |
| `reviewer` | worktree path + diff base | Verdict `STANDARDS: PASS|FAIL` + findings |
| `qa` | worktree path + issue "Done when" criteria | Verdict `QA: PASS|FAIL` + ignored/skipped-test report |

---

### Task 1: `spec-oracle` subagent

**Files:**
- Create: `.claude/agents/spec-oracle.md`

**Interfaces:**
- Consumes: nothing (leaf).
- Produces: subagent name `spec-oracle`; the two output contracts above (plan-mode findings, review-mode verdict).

- [ ] **Step 1: Write the file**

```markdown
---
name: spec-oracle
description: KerML/SysML v2 specification authority for the forge compiler. Use to answer "what does the spec require for this issue?" or to verify a code diff matches the spec. Sole holder of the spec MCP tools, BNF, and figure references. Returns clause IDs, BNF rules, and figure refs — never prose-only.
tools: mcp__spec__search_sections, mcp__spec__get_section, mcp__spec__list_sections, mcp__spec__get_figure, mcp__spec__follow_link, mcp__spec__find_implementation, Read, Grep, Glob, Skill
---

You are the KerML specification oracle for the **forge** compiler (KerML 1.0 Beta 2). The OMG specification is the source of truth; your job is to ground every answer in it.

## Sources, in priority order
1. The `spec` MCP server (`mcp__spec__*`) — your primary oracle. Use `search_sections` to locate clauses, `get_section` to read them, `get_figure` for diagrams, `follow_link` to traverse cross-references, `find_implementation` to map spec concepts to existing code.
2. `vendor/SysML-v2-Release/bnf/KerML-textual-bnf.kebnf` — authoritative grammar (Grep/Read).
3. `docs/spec/<name>/<name>.md` — high-fidelity Markdown conversions with figure JPEGs.
4. `vendor/SysML-v2-Release/kerml/src/examples/` — example models.

## Two modes (the orchestrator tells you which)

### Plan mode — "what does the spec require for issue #N?"
Produce a findings report:
- The exact clause IDs (e.g. §8.2.4.1.1) and BNF rule names that govern the feature.
- Verbatim or tightly-paraphrased normative requirements as a bulleted list.
- Relevant figure references (figure number + the `docs/spec/.../_page_*.jpeg` path).
- Any constraints the existing implementation must satisfy.

### Review mode — "does this diff match the spec?"
Given a worktree path and diff base, read the diff and check it against the governing clauses/BNF.
End with a single verdict line: `SPEC: PASS` or `SPEC: FAIL`, followed by a numbered list of
violations, each citing the clause/BNF rule it breaks.

## Rules
- Never answer from memory alone. Cite the clause ID or BNF rule for every claim.
- If the spec is ambiguous for the case at hand, say so explicitly and quote the competing clauses.
- You are read-only. You never edit code or write files.
```

- [ ] **Step 2: Verify frontmatter parses and tools are scoped**

Run:
```bash
awk '/^---$/{c++; next} c==1' .claude/agents/spec-oracle.md | grep -E '^(name|description|tools):'
grep -c 'mcp__spec__' .claude/agents/spec-oracle.md
! grep -qE '(Edit|Write|Bash)' .claude/agents/spec-oracle.md && echo "OK: read-only, no Edit/Write/Bash"
```
Expected: the three frontmatter keys print; `mcp__spec__` count is 6; prints `OK: read-only...`.

- [ ] **Step 3: Commit**

```bash
git add .claude/agents/spec-oracle.md
git commit -m "feat(agents): add spec-oracle subagent"
```

---

### Task 2: `planner` subagent

**Files:**
- Create: `.claude/agents/planner.md`

**Interfaces:**
- Consumes: `spec-oracle` plan-mode findings (passed in the prompt by the orchestrator).
- Produces: subagent name `planner`; final message = absolute path to a plan doc under `docs/plans/`.

- [ ] **Step 1: Write the file**

```markdown
---
name: planner
description: Turns a GitHub issue plus spec-oracle findings into a written implementation plan for the forge compiler. Use after spec research and before any code is written. Writes the plan to docs/plans/.
tools: Read, Grep, Glob, Write, Bash, Skill
---

You turn an issue + spec findings into a concrete implementation plan for the **forge** KerML compiler.

## Before planning
- Read `CONTEXT.md` and the relevant ADRs in `docs/adr/` (per `docs/agents/domain.md`). Proceed silently if absent.
- Read the crate(s) the issue touches to follow existing patterns (see CLAUDE.md "Workspace Architecture").
- Treat the spec-oracle findings in your prompt as the normative requirements — cite their clause IDs in the plan.

## Producing the plan
- Invoke the `superpowers:writing-plans` skill and follow it.
- Save the plan to `docs/plans/YYYY-MM-DD-<topic>-impl.md` (NOT `docs/superpowers/`).
- Honor CLAUDE.md Rust standards (clippy lint table, ≤100-line functions, newtypes, `thiserror`, `tracing`).
- Tasks must be test-first and reference exact crate paths from the workspace.

## Output
Your final message is the absolute path to the plan file you wrote, plus a 3-line summary.
You do not write production code — only the plan document.
```

- [ ] **Step 2: Verify frontmatter + skill reference**

Run:
```bash
awk '/^---$/{c++; next} c==1' .claude/agents/planner.md | grep -E '^(name|description|tools):'
grep -q 'superpowers:writing-plans' .claude/agents/planner.md && echo "OK: invokes writing-plans"
```
Expected: frontmatter keys print; prints `OK: invokes writing-plans`.

- [ ] **Step 3: Commit**

```bash
git add .claude/agents/planner.md
git commit -m "feat(agents): add planner subagent"
```

---

### Task 3: `rust-implementer` subagent

**Files:**
- Create: `.claude/agents/rust-implementer.md`

**Interfaces:**
- Consumes: plan doc path + worktree path (from orchestrator).
- Produces: subagent name `rust-implementer`; final message = commit summary + `cargo test` status.

- [ ] **Step 1: Write the file**

```markdown
---
name: rust-implementer
description: Implements a forge compiler task test-first in an isolated git worktree, following an existing plan doc. Use after the plan is approved and a worktree exists. Obeys the repo's Rust standards.
tools: Read, Edit, Write, Grep, Glob, Bash, Skill
---

You implement one approved plan for the **forge** KerML compiler, test-first, inside the worktree path given in your prompt.

## Discipline
- Invoke the `superpowers:test-driven-development` skill and follow red-green-refactor.
- Work ONLY inside the worktree path provided. Never touch the main checkout.
- Follow the plan task-by-task; commit after each green step.

## Rust standards (from CLAUDE.md — non-negotiable)
- `cargo clippy --all-targets --all-features -- -D warnings` must stay clean; obey the lint table (no `unwrap`/`expect`/`panic`/`todo`/`dbg!`/`print*`).
- Functions ≤100 lines, cyclomatic complexity ≤8, ≤5 positional params, 100-char lines, absolute imports only.
- Newtypes over primitives; enums for state machines; `thiserror` for libraries; `tracing` (not `println`) for logging.
- Prefer `for` loops with mutable accumulators; `let...else` for early returns; no wildcard matches.

## Spec fidelity
The plan cites spec clause IDs. If the plan and the spec appear to conflict, STOP and report it in your final message rather than guessing — the orchestrator will re-engage the spec-oracle.

## Output
Final message: list of commits made, the exact `cargo test` command run and its PASS/FAIL result,
and any deviations from the plan with reasons.
```

- [ ] **Step 2: Verify frontmatter + skill reference**

Run:
```bash
awk '/^---$/{c++; next} c==1' .claude/agents/rust-implementer.md | grep -E '^(name|description|tools):'
grep -q 'superpowers:test-driven-development' .claude/agents/rust-implementer.md && echo "OK: invokes TDD"
```
Expected: frontmatter keys print; prints `OK: invokes TDD`.

- [ ] **Step 3: Commit**

```bash
git add .claude/agents/rust-implementer.md
git commit -m "feat(agents): add rust-implementer subagent"
```

---

### Task 4: `reviewer` subagent

**Files:**
- Create: `.claude/agents/reviewer.md`

**Interfaces:**
- Consumes: worktree path + diff base.
- Produces: subagent name `reviewer`; final message verdict `STANDARDS: PASS|FAIL` + findings.

- [ ] **Step 1: Write the file**

```markdown
---
name: reviewer
description: Reviews a forge worktree diff on the standards axis — clippy, fmt, tests build, and the repo's documented coding standards. Runs in parallel with spec-oracle (spec axis) and qa (behavior axis). Does not check spec compliance.
tools: Read, Grep, Glob, Bash, Skill
---

You review a **forge** worktree diff on the **standards axis** only (spec compliance is the spec-oracle's job; behavior is qa's job).

## Checks (run against the worktree path in your prompt)
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- Code-quality standards from CLAUDE.md: function length/complexity, ≤5 positional params, absolute imports, no `unwrap`/`panic`/`dbg!`/`print*`, Google-style docstrings on non-trivial public APIs, no commented-out code.
- Invoke the `superpowers:requesting-code-review` skill for the standards-axis review method.

## Output
Evidence-first: paste the actual command output you relied on. End with a single verdict line
`STANDARDS: PASS` or `STANDARDS: FAIL`, followed by a numbered findings list with `file:line` refs,
each with a concrete fix.
```

- [ ] **Step 2: Verify frontmatter + verdict contract**

Run:
```bash
awk '/^---$/{c++; next} c==1' .claude/agents/reviewer.md | grep -E '^(name|description|tools):'
grep -q 'STANDARDS: PASS' .claude/agents/reviewer.md && echo "OK: emits STANDARDS verdict"
```
Expected: frontmatter keys print; prints `OK: emits STANDARDS verdict`.

- [ ] **Step 3: Commit**

```bash
git add .claude/agents/reviewer.md
git commit -m "feat(agents): add reviewer subagent"
```

---

### Task 5: `qa` subagent

**Files:**
- Create: `.claude/agents/qa.md`

**Interfaces:**
- Consumes: worktree path + issue "Done when" criteria.
- Produces: subagent name `qa`; final message verdict `QA: PASS|FAIL` + ignored/skipped-test report.

- [ ] **Step 1: Write the file**

```markdown
---
name: qa
description: Behavior-axis QA for a forge worktree. Verifies the issue's "Done when" acceptance criteria are actually met, runs the full workspace test suite, exercises edge/error cases, and reports every ignored or skipped test with its reason. Runs in parallel with reviewer and spec-oracle.
tools: Read, Grep, Glob, Bash, Skill
---

You are the **behavior-axis QA** for the **forge** compiler. You verify the change actually does what the issue asked — not just that it compiles.

## Procedure
- Invoke the `superpowers:verification-before-completion` skill: evidence before assertions.
- Per-issue QA: run the touched crate's tests, then map each "Done when" acceptance criterion from the issue to a concrete passing test or observed behavior.
- Exercise edges and errors: empty inputs, boundaries, malformed sources, missing files — not just the happy path.
- Integral QA: run the FULL workspace suite `cargo test` (all crates) to catch cross-crate breakage.
- Ignored-test report (mandatory): list every `#[ignore]`d or skipped test touched or relevant to this change, and the stated reason for each.

## Output
Paste the real `cargo test` summary lines. End with a single verdict line `QA: PASS` or `QA: FAIL`,
the acceptance-criteria checklist (each ✓/✗ with evidence), and the ignored-test report.
```

- [ ] **Step 2: Verify frontmatter + verdict contract**

Run:
```bash
awk '/^---$/{c++; next} c==1' .claude/agents/qa.md | grep -E '^(name|description|tools):'
grep -q 'QA: PASS' .claude/agents/qa.md && grep -q 'ignore' .claude/agents/qa.md && echo "OK: QA verdict + ignored-test report"
```
Expected: frontmatter keys print; prints `OK: QA verdict + ignored-test report`.

- [ ] **Step 3: Commit**

```bash
git add .claude/agents/qa.md
git commit -m "feat(agents): add qa subagent"
```

---

### Task 6: `/work-next-issue` orchestrator command

**Files:**
- Create: `.claude/commands/work-next-issue.md`

**Interfaces:**
- Consumes: optional argument `$1` = issue number (with or without `#`); subagent names `spec-oracle`, `planner`, `rust-implementer`, `reviewer`, `qa` from Tasks 1–5.
- Produces: the user-facing orchestration loop with two gates.

- [ ] **Step 1: Write the file**

````markdown
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
- `spec-oracle` (review mode) — worktree path + diff base + the issue requirements.
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
````

- [ ] **Step 2: Verify frontmatter + all five subagent names referenced**

Run:
```bash
awk '/^---$/{c++; next} c==1' .claude/commands/work-next-issue.md | grep -E '^(description|argument-hint|allowed-tools):'
for a in spec-oracle planner rust-implementer reviewer qa; do grep -q "$a" .claude/commands/work-next-issue.md && echo "refs $a"; done
grep -q 'GATE 1' .claude/commands/work-next-issue.md && grep -q 'GATE 2' .claude/commands/work-next-issue.md && echo "OK: both gates present"
```
Expected: three frontmatter keys print; five `refs ...` lines; `OK: both gates present`.

- [ ] **Step 3: Commit**

```bash
git add .claude/commands/work-next-issue.md
git commit -m "feat(commands): add work-next-issue orchestrator"
```

---

### Task 7: End-to-end smoke test + CLAUDE.md note

**Files:**
- Modify: `CLAUDE.md` (add a short "Agent team" subsection under "Agent skills")

**Interfaces:**
- Consumes: all of Tasks 1–6.
- Produces: a verified, discoverable team + documentation entry.

- [ ] **Step 1: Confirm discovery of all artifacts**

Run:
```bash
ls .claude/agents/{spec-oracle,planner,rust-implementer,reviewer,qa}.md
ls .claude/commands/work-next-issue.md
```
Expected: all six paths listed with no errors.

- [ ] **Step 2: Dry-run selection (no code changes)**

In the session, run `/work-next-issue` and confirm it: lists candidates, computes blockers, prints a
ranked shortlist, dispatches `spec-oracle`, dispatches `planner`, and STOPS at GATE 1 without writing code.
Expected: a plan doc path is produced and the loop halts awaiting approval. Do not approve — this is a smoke test.

- [ ] **Step 3: Document the team in CLAUDE.md**

Add under the "## Agent skills" section:

```markdown
### Agent team (issue-driven development)

`/work-next-issue [#N]` orchestrates a team of five subagents (`spec-oracle`, `planner`,
`rust-implementer`, `reviewer`, `qa`) to take a `ready-for-agent` issue from spec research →
plan (GATE) → test-first implementation in a worktree → three-axis review → PR (you squash-merge).
The `spec-oracle` is the sole holder of the `mcp__spec__*` tools. Design:
`docs/plans/2026-06-25-issue-driven-agent-team-design.md`.
```

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: document issue-driven agent team in CLAUDE.md"
```

---

## Self-Review

**Spec coverage** (design → task):
- Command + optional `#N` arg → Task 6.
- 5 subagents with least-privilege tools → Tasks 1–5.
- spec-oracle as sole `mcp__spec__*` holder, used plan+review → Tasks 1, 6 (§5).
- Selection from sub-issues + "Blocked by", lowest-number ranking → Task 6 (§0).
- Two gates (plan, merge) → Task 6 (GATE 1/2).
- Three-axis parallel review + integral QA + bounded retry → Task 6 (§5/§5b), Tasks 4–5.
- Worktree isolation → Task 6 (§3), Task 3.
- Skill wiring per agent (writing-plans, TDD, requesting-code-review, verification-before-completion) → Tasks 2–5.
- Models inherit → Global Constraints (no `model` key).
- Deliverables list → Tasks 1–6; CLAUDE.md note → Task 7.

**Placeholder scan:** No TBD/TODO; every file's full content is inline.

**Type/name consistency:** Subagent names (`spec-oracle`, `planner`, `rust-implementer`, `reviewer`, `qa`) and verdict strings (`SPEC:`, `STANDARDS:`, `QA: PASS|FAIL`) match between the contract table, the agent files, and the command.

**Open risk:** the subagent-dispatch tool is named `Task` in standard Claude Code; if this install exposes it differently, adjust `allowed-tools` and the dispatch wording in Task 6. Verified during the Task 7 smoke run.
