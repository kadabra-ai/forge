# Issue-Driven Agent Team — Design

Date: 2026-06-25
Status: Approved (design)

## Purpose

Build and test features specified in GitHub issues of `kadabra-ai/forge`, driven by an
orchestrator command and a team of five specialist subagents. The team picks the next
`ready-for-agent` issue (respecting dependencies), researches the KerML specification as an
oracle, plans, implements test-first in an isolated worktree, then verifies along three
independent axes (standards, spec, behavior) before opening a PR for the user to squash-merge.

The defining constraint of this project: **forge is a KerML/SysML v2 compiler, and the OMG
specification is the source of truth.** Spec fidelity is therefore enforced structurally — a
single spec authority (`spec-oracle`) is consulted twice (to inform the plan, and to review the
diff), and it is the only agent holding the spec MCP tools.

## Driving model

- **Orchestrator + manual specialists.** One command drives the full loop; each specialist
  subagent is also independently dispatchable for a single step.
- **The orchestrator is a slash command running in the main session — not a subagent.** Only the
  main session can pause for user-approval gates and dispatch subagents. Subagents run in
  isolation, return one final message, and cannot dispatch further subagents.

## Command

`/work-next-issue [#N]`

- `/work-next-issue #N` — work that specific issue. Guard first: confirm it is `ready-for-agent`
  and unblocked; if blocked, warn and list blockers before proceeding.
- `/work-next-issue` — auto-select the top-priority unblocked `ready-for-agent` issue (see
  Selection).

## Roster

The orchestrator (the command, in the main session) dispatches five subagents in `.claude/agents/`.

| Agent | Role | Tools (least privilege) | Phase |
|---|---|---|---|
| `/work-next-issue` (main session) | Selection, dispatch, gates, worktree management, PR | Agent, Bash(gh/git), Read | whole loop |
| `spec-oracle` | KerML authority — the only holder of spec tools. Answers "what does the spec require?" and "does this diff match the spec?" | `mcp__spec__*`, Read, Grep, Glob, Bash(read-only search of `bnf/`, `docs/spec/`, `vendor/`). **No Edit/Write.** | plan + review |
| `planner` | Turns issue + spec findings into a design/plan via `superpowers:writing-plans` | Read, Grep, Glob, Write (plan doc only), Bash(git log) | plan |
| `rust-implementer` | Implements test-first in an isolated worktree via `superpowers:test-driven-development` | Read, Edit, Write, Bash(cargo) | implement |
| `reviewer` | Standards axis: clippy/test/fmt + `/review` standards | Read, Grep, Glob, Bash(cargo) | review |
| `qa` | Behavior axis: full test suite, issue acceptance criteria, ignored-test reporting | Read, Grep, Glob, Bash(cargo) | review + integral QA |

Spec compliance is checked twice (plan-time and diff-time), both by `spec-oracle`.

## Selection & dependency resolution

When no issue number is given:

1. **Fetch candidates:** `gh issue list --state open --label ready-for-agent --json number,title,body,labels`.
2. **Build the blocked-by graph** from two sources (both present in this repo's issues):
   - **Native sub-issues** — `gh api repos/{owner}/{repo}/issues/<n>/sub_issues`. A parent (e.g.
     #7, #13) is blocked until all its children close — parents are tracking/epic issues, not
     directly workable.
   - **Free-text "Blocked by"** — parse the `## Blocked by` section of each body for `#<n>`
     references. "None" / empty = unblocked.
3. **Filter to unblocked leaves:** keep issues whose every blocker is closed and that have no open
   sub-issues.
4. **Rank:** lowest issue number first (matches how the sub-issue chains are authored in order,
   e.g. #21→#27). Print the ranked shortlist so the user can override.

**Override:** the user re-runs with an explicit `#N`, or tells the orchestrator which to do first.
No custom Project field is used — manual kanban drag-order is not API-stable; lowest-number-first
plus explicit override is sufficient.

## Flow & gates

```
/work-next-issue [#N]
│
├─ 0. SELECT   resolve issue (Selection) + guard (ready-for-agent, unblocked)
│
├─ 1. SPEC     dispatch spec-oracle(issue) -> spec findings (clause IDs, BNF rules, figure refs)
│
├─ 2. PLAN     dispatch planner(issue + spec findings) -> design/plan in docs/plans/
│              === GATE 1: show plan, wait for user approval ===
│
├─ 3. SETUP    create worktree for the issue (superpowers:using-git-worktrees)
│
├─ 4. BUILD    dispatch rust-implementer(plan, worktree) -> TDD commits in worktree
│
├─ 5. REVIEW   dispatch IN PARALLEL, all reading the worktree diff:
│                • reviewer    -> standards axis
│                • spec-oracle -> spec axis (diff vs spec/BNF/diagrams)
│                • qa          -> behavior axis (acceptance criteria, edge/error cases)
│
├─ 5b. INTEGRAL QA   qa: full-workspace `cargo test` (catch cross-crate breakage)
│      any axis fails -> loop to 4 with findings (bounded: max 2 rounds, then escalate)
│      ignored/skipped tests surfaced to the user with reasons
│
├─ 6. PR       push branch, open PR ("Closes #N"), link the plan doc
│              === GATE 2: stop. user squash-merges ===
│
└─ (after merge) issue auto-closes via "Closes #N"; worktree cleaned up
```

- **Gates:** GATE 1 after spec+plan (before any code); GATE 2 before merge (never auto-merge).
  Everything between runs autonomously, including opening the PR.
- **Worktree isolation:** each issue gets its own worktree under the existing `.claude/worktrees/`;
  the implementer never touches the main checkout.
- **Bounded retry:** review failures feed findings back to the implementer, capped at 2 rounds,
  then surfaced to the user rather than looping.

## Skill & spec wiring per agent

- **spec-oracle** — primary oracle is `mcp__spec__*` (`search_sections`, `get_section`,
  `get_figure`, `follow_link`, `find_implementation`); cross-checks
  `vendor/SysML-v2-Release/bnf/KerML-textual-bnf.kebnf` and `docs/spec/`. Returns clause IDs, BNF
  rules, and figure references — never prose-only. Read-only.
- **planner** — invokes `superpowers:writing-plans`; reads `CONTEXT.md` + `docs/adr/` first (per
  `docs/agents/domain.md`); writes to `docs/plans/YYYY-MM-DD-<topic>-*.md`.
- **rust-implementer** — invokes `superpowers:test-driven-development`; obeys CLAUDE.md Rust rules
  (clippy lint table, <=100-line fns, newtypes over primitives, `tracing` not `println`, no
  `unwrap`/`panic`); works only in its worktree.
- **reviewer** — invokes `superpowers:requesting-code-review` / the `/review` standards axis.
- **qa** — invokes `superpowers:verification-before-completion`; evidence-before-assertions (pastes
  real `cargo test` output); reports every `#[ignore]`d or skipped test and the reason.

## Models

Inherit (no per-agent override). The user will test `sonnet` for `rust-implementer` later.

## Deliverables

```
.claude/
├── commands/
│   └── work-next-issue.md          # orchestrator command (optional #N argument)
└── agents/
    ├── spec-oracle.md
    ├── planner.md
    ├── rust-implementer.md
    ├── reviewer.md
    └── qa.md
docs/superpowers/specs/2026-06-25-issue-driven-agent-team-design.md   # this design
```

## Out of scope

- Auto-merge (user always squash-merges).
- Custom GitHub Project priority fields / reading kanban drag-order.
- Triaging issues into `ready-for-agent` (a separate `/triage` concern).
- Multi-issue parallel execution (the loop is one issue at a time).
