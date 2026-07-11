# API Key Routing Customization Handoff

## Scope

This repository maintains a downstream Cockpit Tools customization on
`feature/api-key-routing-usage`.

The customization provides:

- A selectable Codex account pool for each client API Key.
- Inheritance from the service account pool when a Key has no explicit
  account IDs.
- Per-Key request count, token usage, success rate, and estimated cost.
- End-to-end `accountIds` propagation across TypeScript, Tauri commands, and
  Rust persistence.
- Sidecar enforcement, per-Key session-affinity isolation, account-scope
  validation, and compact token display added by later hardening commits.

Do not record passwords, API Keys, OAuth tokens, signing keys, or other
credentials in this document, patches, commits, logs, or pull requests.

## Archived State

State checked before the upstream fetch on 2026-07-11:

- Working branch: `feature/api-key-routing-usage`.
- Initial working tree: clean. `src-tauri/Cargo.toml` had no text diff.
- Initial `HEAD`: `75dbb614` (`docs: add downstream customization maintenance guide`).
- Branch upstream: `fork/feature/api-key-routing-usage`.
- Official remote: `origin` -> `https://github.com/jlcodes99/cockpit-tools.git`.
- Personal fork remote: `fork` -> `https://github.com/kin001/cockpit-tools.git`.
- Official tracking commit before fetch: `93d939b7`.
- Custom package version before fetch: `1.1.7`.
- Requested historical core commit: `2629dc16`.
- Rebased equivalent of that core change in the current branch: `1911dc3d`.

`2629dc16` is not an ancestor of the current branch because the branch was
previously rebased. Its patch is archived at:

```text
docs/maintenance/patches/0001-Add-API-key-account-routing-controls.patch
```

That patch contains only the original five-file implementation. The complete
current behavior also depends on the later commits from `03bd0d67` through
`96a8e096`; restoring only the archived patch does not restore all hardening,
sidecar, compatibility, or display changes.

Local untracked material under `docs/superpowers/plans` is not part of the
official contribution and must not be deleted or included in an upstream PR.

## Key Files

- `src-tauri/src/commands/codex.rs`
- `src-tauri/src/models/codex_local_access.rs`
- `src-tauri/src/modules/codex_local_access.rs`
- `src/services/codexLocalAccessService.ts`
- `src/types/codexLocalAccess.ts`
- `src/pages/CodexApiServicePage.tsx`
- `src/pages/CodexApiServicePage.css`
- `src/utils/codexApiKeyAccountScope.ts`
- `sidecars/cockpit-cliproxy/main.go`
- `sidecars/cockpit-cliproxy/cdk/CLIProxyAPI/sdk/cliproxy/auth/selector.go`

## Recovery

Preferred full recovery uses the personal fork because it contains the whole
hardened branch:

```powershell
git fetch fork
git switch -c feature/api-key-routing-usage --track fork/feature/api-key-routing-usage
```

To restore only the original core change onto a compatible clean checkout:

```powershell
git switch -c recover/api-key-routing origin/main
git am docs/maintenance/patches/0001-Add-API-key-account-routing-controls.patch
```

If `git am` conflicts, abort it before retrying a manual port:

```powershell
git am --abort
```

For disaster recovery of the exact pre-fetch state, retain the commit ID
`75dbb614` and create a branch without resetting the current worktree:

```powershell
git branch recovery/api-key-routing-pre-fetch 75dbb614
```

Never use `git reset --hard`, `git clean`, or checkout-based file restoration
when uncommitted or untracked files may be present.

## Upstream Upgrade Procedure

1. Inspect and preserve local state:

   ```powershell
   git status --short --branch
   git diff --stat
   git diff
   git log --oneline --decorate -12
   git branch --show-current
   ```

2. Refresh official history and inspect it before merging:

   ```powershell
   git fetch origin
   git log --oneline --decorate HEAD..origin/main
   git diff --stat HEAD...origin/main
   git diff --name-status HEAD...origin/main
   ```

3. Create a backup reference, then merge the official branch:

   ```powershell
   git branch backup/pre-upstream-sync-$(Get-Date -Format yyyyMMdd-HHmmss)
   git merge origin/main
   ```

4. Resolve conflicts while preserving both upstream behavior and every item
   in the Scope section. Pay particular attention to the key files above and
   to version metadata in `package.json`, `package-lock.json`,
   `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and `Cargo.lock`.

5. Review the resolved merge before committing:

   ```powershell
   git status --short
   git diff --check
   git diff --cached --stat
   ```

Do not use the official in-app updater for this checkout. It installs an
official binary without the downstream behavior even though stored settings
remain on disk.

## Verification

Run the required TypeScript and Rust regression checks from the repository
root:

```powershell
npm run typecheck

$env:COCKPIT_SKIP_CLIPROXY_BUILD='1'
Set-Location src-tauri
cargo test -q custom_api_key_scope_filters_duplicates_and_updates_manifest_scope --lib
Set-Location ..
```

The historical test name
`update_api_key_account_scope_filters_duplicates_and_updates_manifest_scope`
no longer matches a registered test after the previous hardening refactor. It
returns success with `0 tests`; do not treat that as regression evidence. The
current equivalent name above must report `1 passed`.

Also run the focused JavaScript tests when their files are present:

```powershell
npm run test:codex-api-key-scope
node --test src/utils/codexApiServiceCompatibility.test.ts
```

The machine has no Go toolchain. Do not require a sidecar rebuild for local
verification; use the existing Windows sidecar binary:

```text
sidecars/cockpit-cliproxy/bin/cockpit-cliproxy-x86_64-pc-windows-msvc.exe
```

## Packaging

There is no `TAURI_SIGNING_PRIVATE_KEY` on this machine. Keep the committed
release configuration unchanged and disable updater artifacts only for the
local build by using the existing CI override:

```powershell
$env:COCKPIT_SKIP_CLIPROXY_BUILD='1'
npm run tauri -- build --config src-tauri/tauri.ci.conf.json
```

`src-tauri/tauri.ci.conf.json` sets `bundle.createUpdaterArtifacts=false` and
therefore avoids the updater-signing requirement. Confirm that the existing
sidecar binary is present before packaging. This workspace uses the
repository-level Cargo target directory. Windows installers are written under:

```text
target/release/bundle/nsis
target/release/bundle/msi
```

After building, record the installer name, size, timestamp, and SHA-256 hash:

```powershell
Get-ChildItem target\release\bundle\nsis,target\release\bundle\msi
Get-FileHash target\release\bundle\nsis\*.exe,target\release\bundle\msi\*.msi -Algorithm SHA256
```

## Publishing

Push the feature branch to the personal fork, never directly to the official
repository:

```powershell
git push fork feature/api-key-routing-usage
```

If history was intentionally rebased, use `--force-with-lease`, not a plain
force push. Open the pull request from
`kin001:feature/api-key-routing-usage` to `jlcodes99:main`. Before publishing,
exclude local planning files and scan the diff for credentials and generated
artifacts.

## Latest Sync Result

Sync performed on 2026-07-11:

- Fetched official commit: `c9e4c856` (`chore(homebrew): update cask for
  v1.1.5 (#1506)`).
- New official tag: `v1.1.5`.
- Merge parents: downstream `75dbb614` and official `c9e4c856`.
- Backup reference: `backup/pre-origin-main-merge-20260711-112957`.
- Version retained: downstream `1.1.7`, newer than official `1.1.5`.
- Conflicts resolved in both changelogs, `Cargo.lock`, `package.json`,
  `sidecars/cockpit-cliproxy/main.go`, `src-tauri/Cargo.toml`,
  `src-tauri/src/modules/codex_local_access.rs`, and
  `src-tauri/tauri.conf.json`.
- Resolution preserved per-Key account scopes, inherited empty scopes, usage
  statistics, sidecar scope enforcement, session isolation, and compact token
  display while adding the official OAuth quota-reserve behavior and release
  updates.
- One official Rust test initializer required the downstream
  `inherit_account_pool` field after the merge.

Verification results:

- `npm run typecheck`: passed.
- `npm run test:codex-api-key-scope`: 5 passed.
- `node --test src/utils/codexApiServiceCompatibility.test.ts`: 2 passed.
- Historical Rust command: compiled successfully but ran 0 tests because the
  old name is no longer registered.
- Current Rust equivalent
  `custom_api_key_scope_filters_duplicates_and_updates_manifest_scope`: 1
  passed.
- `cargo fmt --check`: passed after formatting the merged test imports.
- Go tests: not run because Go is unavailable on this machine.
- Tauri build with `COCKPIT_SKIP_CLIPROXY_BUILD=1` and
  `src-tauri/tauri.ci.conf.json`: passed.

Packaged artifacts:

```text
target/release/bundle/nsis/Cockpit Tools_1.1.7_x64-setup.exe
size: 26325870 bytes
time: 2026-07-11 12:06:12 +08:00
sha256: 559499239038037A209D643C89D7737649509A9085757535F28D876DBA23A527

target/release/bundle/msi/Cockpit Tools_1.1.7_x64_en-US.msi
size: 35926016 bytes
time: 2026-07-11 12:05:14 +08:00
sha256: 86196B21ABCD883556FBEFFD7B6EE11F1E5A91572713CC0D5152E573EA48D3E8
```

The reused sidecar was not rebuilt:

```text
sidecars/cockpit-cliproxy/bin/cockpit-cliproxy-x86_64-pc-windows-msvc.exe
size: 19257344 bytes
time: 2026-07-10 18:09:55 +08:00
sha256: A3DF9E4165E5A15A5684F1F84593DF873013BF6103AFBD02682E19C9B4770DF9
```

Because this binary predates the fetched 2026-07-11 upstream source and Go is
unavailable, this local package does not prove that newly merged upstream
sidecar changes are present in the bundled executable. Rebuild and rerun the
Go tests on a Go-enabled machine before treating the package as a complete
release candidate. The existing binary does contain the previously packaged
downstream API Key scope enforcement.

The build emitted existing Rust unused/dead-code warnings and a Tauri warning
that `__TAURI_BUNDLE_TYPE` was not found while patching MSI/NSIS binaries.
Both local bundles were still produced successfully; updater artifacts were
disabled for this unsigned local build.
