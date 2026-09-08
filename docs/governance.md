# Repository governance

`pixbs/claimlands-astro` uses the rules in `.github/rulesets/`, rebase merges,
resolved review conversations, and manual owner acceptance. Automatic merging is
disabled. `main` requires current `publication-policy`, `quality`, and
`game-preview` checks, linear history, and protection against deletion and force
pushes. There are no routine bypass actors.

## Trusted validation

For pull requests, `validate.yml` runs through `pull_request_target` from `main`.
Candidate game code executes only in jobs with read-only permissions and no
deployment secrets. Policy, aggregate checks, deployment code, browser journeys,
and expected images come from the trusted revision. Workflow and baseline changes
therefore need owner review before they govern subsequent PRs. Testing a proposed
workflow through `workflow_dispatch` is useful evidence, but cannot authorize a
merge through the merge helper.

The owner merge entry point is `python -I tools/merge.py`, run from an isolated,
clean checkout of the verified current GitHub `main` revision. Follow the
[checkout procedure](contributing.md#review-and-acceptance); do not run merge
tooling from the candidate checkout. The entry point rejects feature branches,
stale main revisions, and tracked or untracked changes. It loads policy and
provenance validators directly from that Git revision, avoiding local import
paths and bytecode caches.

After explicit owner acceptance, it checks the entire introduced history,
metadata, actual workflow ID and trigger, completed jobs, run attempt, and
validation manifest. It rechecks main and the approved head before requesting a
rebase merge with an expected-head condition. Missing, skipped, stale, or
unsuccessful evidence is rejected. A green status by itself is insufficient.
The Cargo command delegates to this entry point; it does not make arbitrary
candidate Cargo tooling trustworthy or establish owner approval.

GitHub status rules cannot bind a context to an individual workflow on this
personal repository. The helper verifies that stronger condition; an owner using
another merge route must perform the same review. Shared owner credentials cannot
technically distinguish a person from an agent. A dedicated publishing GitHub App
or an organization with required-workflow controls is the next step if credential
separation becomes necessary.

Validation staging replaces complete Cargo configuration and tooling directories,
so candidate-only config files and automatically discovered targets cannot
survive an overlay. Browser tooling uses a fresh trusted harness, npm manifests,
and lockfile. Submitted build scripts still execute within their build runner;
this does not establish isolation against arbitrary hostile code modifying that
runner or its reports. Deployment credentials remain in the separate publisher
job. Stronger isolation of build execution would require an additional sandbox.

## Publication policy

`AGENTS.md` holds the authoritative prompt header. `tools/policy.py` validates
introduced commit messages, identities, trailers, branch names, and current PR
text. Local hooks apply it before publication; CI reloads policy from trusted
`main` and rechecks edited metadata.

This repository's API rejected metadata-pattern rules. Its active branch/tag
rules instead block creation and updates of refs matching known prohibited names
using case-insensitive character classes. The complete naming convention and
Unicode normalization remain enforced by hooks and PR checks. Keep the explicit
name list in policy and the ref rules synchronized when extending it.

With direct publishing, an offending commit or PR body can reach GitHub before a
check rejects it. These controls prevent acceptance; they do not promise that all
invalid metadata can be prevented from arriving. Never test a prohibited name by
publishing it: use local negative fixtures and inspect the server rules.

## Cloudflare previews

The Pages project is `claimlands-astro`, with production branch `main`. Store
`CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` in the GitHub environment
`game-preview`, restricted to the `main` branch. Do not store deployment secrets
as repository-wide secrets: same-repository feature workflows could read them.
The token needs Pages Edit permission scoped to the deployment account.

Build jobs upload the actual tested Wasm artifact. A separate job validates its
contents and revision, then uploads static files without executing submitted
scripts. PRs use `pr-N`; production updates after an accepted merge. The immutable
deployment URL and revision identify the review build, while the branch alias
tracks later updates. Deployed browser journeys must pass before the preview gate
can succeed. PR previews may be published for review while another gate fails;
production requires every gate to pass.

## Changing these controls

Keep governance changes small and review the exact diff and diagnostic evidence.
Changes to thresholds, exclusions, expected images, workflows, or trust boundaries
need explicit owner acceptance. A broken trusted workflow can block its own repair;
complete and test the proposed correction before requesting a narrowly scoped
owner maintenance decision. Never disable protection or manufacture successful
checks to resolve that condition.
