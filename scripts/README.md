# Project scripts

| Setup | Scripts | Pre-commit install |
|-------|---------|-------------------|
| Linux / WSL | `scripts/linux/*.sh` | `./scripts/linux/setup-git-hooks.sh` |
| Windows | `scripts/windows/*.ps1` or `*.cmd` | `scripts\windows\setup-git-hooks.cmd` |

## Linux / WSL

```bash
./scripts/linux/ci-check.sh
./scripts/linux/setup-git-hooks.sh
./scripts/linux/package-godot-release.sh VERSION linux.so win.dll mac.dylib
./scripts/linux/bump-version.sh
./scripts/linux/sync-version.sh
```


## Windows

```cmd
scripts\windows\ci-check.cmd
scripts\windows\setup-git-hooks.cmd
scripts\windows\bump-version.cmd
scripts\windows\sync-version.cmd
```

PowerShell equivalents:

```powershell
.\scripts\windows\ci-check.ps1
.\scripts\windows\setup-git-hooks.ps1
.\scripts\windows\bump-version.ps1
.\scripts\windows\sync-version.ps1
```

## Versioning

`VERSION` at the repository root is the single source of truth. Workspace crates inherit
`version.workspace = true` from `[workspace.package]` in root `Cargo.toml`. Cargo cannot read
`VERSION` directly; run `sync-version` after editing `VERSION` by hand (updates `Cargo.toml`,
`Cargo.lock`, and `docs/description.md`), or use `bump-version` before a release (syncs
files, commits, and creates `vX.Y.Z`).

## CI

`ci-check` matches the test jobs in [`.github/workflows/rust.yml`](../.github/workflows/rust.yml)
and [`.gitlab-ci.yml`](../.gitlab-ci.yml). See [`docs/ci.md`](../docs/ci.md).

## Git hooks

`.githooks/pre-commit` runs the platform CI script (`ci-check`), including VERSION sync verification.

```bash
./scripts/linux/setup-git-hooks.sh
```

```cmd
scripts\windows\setup-git-hooks.cmd
```
