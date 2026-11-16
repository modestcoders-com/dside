# Spec: the verifier may run read-only commands

## Goal

The verifier today may read files but cannot run anything, so a claim like "the tests pass" can
only be checked by trusting the one who made it. This spec gives it `bash` with the smallest list
of programs that cannot change anything — and makes the boundary a written rule, not an
improvisation.

## Decided

Yes. A fixed list of read-only programs, no redirection, no writing flags. Anything not classified
does not run, and the verifier then says **it could not check** — it never reports a claim as
verified because the check was impossible.

## The list

Read effect only, and nothing that can write by accident:

```
git status, git log, git show, git diff, git branch --list, git rev-parse, git ls-files
grep, rg, ls, find (without -delete/-exec), cat, head, tail, wc, diff, cmp, file, stat, du
```

Rules on top of the names:

- **No redirection at all** (`>`, `>>`, `|` into a file, `2>`, `<` is fine for reading a named file
  but not required). A pipe is allowed only when **every** segment is classified read-only —
  the same scanner that already splits the command line for the allowlist does this.
- **No `$(...)`, no backticks.** This is already refused by the command scanner and stays refused.
- **A deny-list of flags**, because `git branch -D` and `find -delete` are not read-only even
  though their program is: `-delete`, `-exec`, `-execdir`, `-ok`, `--force`, `-f` on `rm`-like
  programs, `-D`/`-d` on `git branch`, and `--output`. A file is written by a flag, not only by a
  redirection.
- **Within the workspace only**, like every other tool. Reading `/etc` is not verification of the
  work.

## Why a list and not a sandbox

A sandbox would be a claim about the kernel and about every program inside it. A list is a claim
about twenty-odd names and a scanner that already exists and is already tested. The second is
reviewable by reading one function, and it fails closed: an unknown program does not run.

The honest limit belongs in the spec: this stops an accident and a careless command, not a program
that lies about its flags. A verifier that wants to cause harm is not the threat this addresses —
the threat is a verifier that believes a claim because checking it looked hard.

## Shape

- Reuse `split_commands` and extend it with a classification: `ReadOnly`, `Refused(reason)`.
- `run_shell_command` keeps its behaviour for the companion's `bash`; the verifier calls a new
  `run_readonly_command` that consults the classifier first.
- The refusal text names the program and the rule, in the same voice as the allowlist refusals:
  "`git branch -D` is not read-only; run it yourself if you want it."
- Output is truncated like every tool result, and the verifier records the command and the
  transcript as evidence.

## What "done" means

- Every program in the list runs; anything else is refused with a reason.
- `git branch -D x`, `find . -delete`, `echo hi > f`, `grep x $(ls)` are refused, each with the
  rule that caught them.
- In read-only permission stage the verifier still runs — reading is what it does.
- Tests: one per rule above, plus a test that a pipe of two classified programs runs and a pipe
  with one unclassified program does not.

## Out of scope

- Write commands with a confirmation step. If the verifier needs to change something, it is not
  verifying.
- A second allowlist for the companion that is looser than the user's.
