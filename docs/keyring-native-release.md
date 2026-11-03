# Fix: Windows/macOS API key is not persisted (v0.4.9)

## Goal

The API key entered on Windows/macOS is reported as saved ("Gespeichert im
OS-Schlüsselbund") but is gone on the next request/restart. Cause: `keyring` 3.x
has **no default features**; with only `sync-secret-service` (Linux) no platform
store applies on Windows/macOS, so the crate silently uses its **mock** store.
The mock keeps the value only inside the `Entry` that wrote it, so a fresh entry
in `provider_request` is empty → `API key is missing`.

The fix enables the native stores, verifies the write, and makes the existing
file fallback actually reachable. The prepared handover patch is in
`_handover/dside-fix.patch` (against `f5eeda7` / v0.4.8); the current tree still
matches its pre-image.

## Scope

In scope:

- `keyring` features `apple-native` and `windows-native` (keep
  `sync-secret-service` + `crypto-rust` for Linux).
- Read-back verification in `SecretStore::set` (the analysis recommends it; the
  handover patch does **not** include it).
- Two-entry keyring test from the handover patch.
- Handover patch's provider-client reuse, one retry on transport errors, error
  chain in messages, and serial `loadModels`/`loadBalance`.
- Windows-safe `skills.rs` path assertion (test only).
- v0.4.9 version bump, PROGRESS note, release/tag, draft cleanup.
- Optional but recommended: run `cargo test --lib` on Windows/macOS in the
  release workflow, because that is where the mock bug was invisible.

Out of scope for this release:

- Editor picker "scharf schalten" (planned for Friday).
- Account pill "modest" / Whale + DeepSeek branding.
- KDE/Shelly desktop integration.
- Browser/Monaco panel behavior and the mode switch investigation.
- Memory search.

## Tasks

1. **Apply the handover patch**

   ```sh
   git apply --check _handover/dside-fix.patch
   git apply _handover/dside-fix.patch
   # if the tree drifted: git apply --3way _handover/dside-fix.patch
   ```

   Files: `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`,
   `src-tauri/src/secret.rs`, `src-tauri/src/skills.rs`,
   `src-tauri/src/engine/mod.rs`, `src/components/settings/SettingsPage.vue`.

   Do not commit `_handover/` or `docs/markdown-test.md`.

2. **Regenerate the lockfile after the feature change**

   ```sh
   cd src-tauri
   cargo check
   cargo test --lib
   ```

   Keep the updated `Cargo.lock` (keyring pulls `security-framework`,
   `windows-sys`, `byteorder` for the target-specific backends). If the patch's
   lock hunks conflict, discard only the `Cargo.lock` changes and let Cargo
   rewrite them.

3. **Harden `SecretStore::set`**

   In `src-tauri/src/secret.rs`, after `entry.set_password(trimmed).is_ok()`:

   - Create a **fresh** `keyring::Entry::new(SERVICE, ACCOUNT)` and read the
     password back.
   - If it returns exactly `trimmed`: remove the fallback file and return
     `{ present: true, backend: "keyring" }`.
   - If it returns a different value: delete that credential and use the file
     fallback (do not leave a wrong value in the OS store).
   - If the read fails: keep the entry (it may be transient), write the file
     fallback, return `{ present: true, backend: "file" }`.
   - Only remove the fallback **after** successful verification.

   This catches the mock store and any backend that reports success without
   persistence. `get`/`status` already prefer the keyring and then the file, so
   no further changes are needed there.

4. **Keep the two-entry test**

   The patched `keyring_roundtrip_or_skip` writes through one entry and reads
   through a second one. It must still skip cleanly when no keyring is reachable
   (`set_password` error), but fail when the mock accepts the write.

5. **Recommended CI coverage**

   In `.github/workflows/release.yml`, add a `cargo test --lib` step to the
   Windows and macOS build jobs (before `tauri build`). This is the only place
   the regression was invisible: the Linux `verify` job cannot exercise the
   Windows/macOS stores, and the two-entry test would otherwise never run there.
   Keep the test's skip-on-unavailable behavior so a locked CI keychain does not
   fail the release.

6. **Version and docs**

   ```sh
   node scripts/set-version.mjs 0.4.9
   cd src-tauri && cargo test --lib && cd ..
   pnpm build
   ```

   - Update `PROGRESS.md` with a section for the keyring root cause, the
     read-back hardening, the provider retry/serialization, and the release
     plan (v0.4.9, old draft cleanup).
   - No i18n changes: the existing "keyring" / "file" status strings are correct
     once the backend is real.

7. **Validate locally (Linux)**

   - `pnpm build` (vue-tsc + vite) is green.
   - `cd src-tauri && cargo test --lib` → 24 passed, 12 ignored; note whether
     the keyring test skipped.
   - `pnpm tauri build` (or at least `cargo build --release --features
     tauri/custom-protocol`) succeeds.
   - In the dev app: enter the key, confirm the backend message, restart, and
     confirm models/balance still load. On Linux this already worked; the test
     is a regression check.

8. **Commit, tag, push**

   Suggested commit message:

   ```text
   fix(keyring): use native Windows/macOS stores and verify writes

   keyring 3 has no default features, so Windows/macOS silently used the mock
   store and lost the API key after the entry dropped. Enable apple-native and
   windows-native, read the value back through a fresh entry, and fall back to
   the file if it cannot be verified. Also reuse one provider client, retry a
   transport error once, load models and balance serially, and make the skills
   path assertion platform-independent.
   ```

   Then:

   ```sh
   git add <the six source files, Cargo.lock, PROGRESS.md, package.json,
            src-tauri/Cargo.toml, src-tauri/tauri.conf.json, workflow if changed>
   git commit
   git push origin main
   git push github main
   git tag -a v0.4.9 -m "DS.IDE 0.4.9"
   git push origin v0.4.9
   git push github v0.4.9
   ```

   The tag starts the release workflow (verify → Linux/macOS/Windows → draft).

9. **Acceptance test on Windows 11**

   Download the v0.4.9 Windows asset from the draft (`.exe` or `.zip`), then:

   - Enter the API key; the status must say "Gespeichert im OS-Schlüsselbund"
     (or explicitly "Datei" if the native store is unavailable — never a false
     keyring success).
   - Restart the app; the key must still be present, and models/balance must
     load.
   - Send a chat message; the companion must answer.
   - Optionally confirm the credential exists in Windows Credential Manager
     under `ds-ide.deepvibe` / `deepseek-api-key`.

   If the native store fails on the test machine, the fallback must still make
   the app usable and the UI must name the file backend. That is a pass for the
   key, but report the native failure separately.

10. **Publish and clean up**

    - After the Windows smoke test passes, publish the v0.4.9 draft.
    - Delete the old v0.4.8 **draft release** (its Windows/macOS assets contain
      the bug). Keep the `v0.4.8` tag for history unless there is a reason to
      remove it; it still serves the two source archives.
    - Do not delete v0.4.9 until it has been downloaded/tested.

## Validation checklist

- [ ] `pnpm build` green.
- [ ] `cargo test --lib` green on Linux; keyring test behavior noted.
- [ ] `Cargo.lock` contains the native backend crates and is committed.
- [ ] CI on v0.4.9 green for Linux, macOS, Windows.
- [ ] Draft contains the expected seven assets (or the same set as v0.4.8).
- [ ] Windows: key survives restart and the companion answers.
- [ ] Old v0.4.8 draft removed; v0.4.9 published only after the test.

## Risks / notes

- **Second keychain prompt:** reading the value back immediately may prompt once
  more on macOS. If the write cannot be verified, the file fallback keeps the
  app working and the UI reports "file".
- **Fallback security:** on Windows the fallback file has no Unix `0600`
  equivalent; it is a last resort and is surfaced in the UI. Do not silently
  prefer it over a working native store.
- **No automatic migration:** nothing was persisted by the mock, so the user
  must re-enter the key once after updating. The old "saved" message was false.
- **Feature build cost:** `apple-native`/`windows-native` only compile on their
  targets, but add platform dependencies to the lockfile and slightly increase
  binary size.
- **Provider changes are independent:** the shared client, retry and serial
  loading address the "error sending request"/connection-reset symptom. They do
  not fix the keyring bug, but they are part of the same handover and should be
  kept together for v0.4.9.
- **Do not publish v0.4.8:** its Windows/macOS artifacts are known-broken.

## Open questions

None blocking. The editor picker and the account pill remain scheduled for a
later release.
