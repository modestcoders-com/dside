# Spec: a reviewer before anything leaves the house

## Goal

A check the companion cannot skip, that stands between "the round is done" and "the tag is set".
Not a colleague with an opinion — a gate with an exit code.

## The problem it addresses, concretely

Three failures of the same shape happened in one week, and all three were confident:

- A claim that the browser screenshot tool was missing, when it had existed for weeks.
- A claim that `createUpdaterArtifacts` was the way to give the AppImage updates; it builds the
  project's own updater and needs a signing key, and the real answer was a `zsync` step.
- A repair that was undone minutes later by a second code path, and called fixed before that was
  seen.

None of these were uncertainty. Each was certainty without evidence, which is the only kind of
wrong answer that leaves the house.

## Decided

- **A process, not a faculty.** The companion cannot grow a second opinion; it can call a script
  that refuses to let a release through. The gate lives in the release path, like
  `check-private.mjs` does today.
- **Independence comes from information, not from a second brain.** The reviewer sees the change,
  the claim and the evidence. It does **not** see the reasoning that produced them — the prose
  explaining why the change is right is exactly what makes a reader agree.
- **"Undecided" blocks as well.** Otherwise the gate becomes a rubber stamp for anything it cannot
  be bothered to check.
- **It must be able to stop a release.** A reviewer that can only comment is decoration.

## The input

Given to the reviewer as files, not as conversation:

| File | Content |
| --- | --- |
| `diff.patch` | `git diff <last tag>..HEAD` — the actual change, nothing about it |
| `claims.txt` | one line per claim the round makes ("the context ring shows 4% after compacting", "the package contains `Categories=Development;`") |
| `evidence.txt` | the commands that were run and their output, verbatim |
| `checklist.txt` | what the project requires before a tag (`pnpm build`, `cargo test --lib`, packages started, `.zsync` on the release page, …) |

Deliberately absent: `PROGRESS.md` as narrative, the chat, and any explanation written by the one
whose work is being reviewed.

## The output

A file with one of three verdicts and, for each claim, whether it is **backed**, **contradicted**
or **unbacked**, plus for unbacked ones the command that would settle it.

```
VERDICT: FAIL
- claim: "the updater works end to end"
  state: contradicted
  because: evidence.txt shows the offer was seen, not that an update was performed
- claim: "the .deb carries the category"
  state: backed
  by: evidence.txt:12
```

Rules the reviewer follows, and that are worth writing down because they are the whole point:

- A claim with no command behind it is **unbacked**, however plausible the diff looks.
- "The tests pass" is only backed by the test output of *this* tree.
- A green build proves the code compiles. It never proves behaviour.
- A number in the chronicle is backed only by a measurement that names what was measured.
- When the evidence is ambiguous, the verdict is UNDECIDED, with the specific ambiguity named.

## The gate

- `scripts/review-release.sh` builds the four input files and runs the reviewer, writes
  `review-<version>.md`, exits non-zero on `FAIL` or `UNDECIDED`.
- The release checklist calls it before `git tag`. A failed review is reported to the user with the
  reviewer's reasons verbatim — not paraphrased into something softer.
- The verdict file is kept next to the release, and its objections go into the chronicle, quoted.

## Who reviews

**Our own model, in a separate session with a clean context.** That is the honest shape: the
independence is informational — the reviewer has not read the reasoning and cannot be talked into
agreement by a well-turned paragraph. It is also the only option that costs nothing and needs no
new dependency.

**A free-tier model as the gate is rejected.** The task is judging whether evidence supports a
claim; a weak model agrees with confident prose, which is the failure being fixed. Free test models
are useful for a second pair of eyes on wording or i18n, never for the decision to publish.

## What "done" means

- A round whose claims have evidence passes; the same round with the evidence file emptied fails,
  naming every claim.
- The gate refuses to run when the working tree is dirty beyond the release commit, because then
  the diff is not the thing being shipped.
- A test of the checker itself: a fabricated claim list gives `FAIL`.

## Out of scope

- Judging design decisions ("should the plugin store exist"). The reviewer checks claims against
  evidence, nothing else.
- Reading the chat, the memory, or any file outside the four inputs.
- Later rounds may extend it to destructive commands and deletions; not in the first version.
