# Continuous integration

This repository includes CI configs for **GitHub Actions** and **GitLab CI**. Use the one that
matches your git host; you can delete the other if you only use a single platform.

Local checks mirror both pipelines:

```bash
./scripts/linux/ci-check.sh
# or: scripts\windows\ci-check.cmd
```

## If using GitHub

Config: [`.github/workflows/rust.yml`](../.github/workflows/rust.yml) and
[`.github/workflows/release.yml`](../.github/workflows/release.yml).

| Event | Pipeline |
|-------|----------|
| Push or PR to `main` | `rust.yml` — fmt, clippy, build, test |
| Push tag `vX.Y.Z` on `main` | `release.yml` — cross-platform GDExtension build, zip packaging, GitHub Release |

After `bump-version` and pushing the tag:

```bash
git push origin HEAD
git push origin v0.1.0
```

Release assets appear under **GitHub Releases** for the tag.

## If using GitLab

Config: [`.gitlab-ci.yml`](../.gitlab-ci.yml).

| Event | Jobs |
|-------|------|
| Merge request or push to default branch | `rust-check` — same steps as `ci-check.sh` |
| Push tag `vX.Y.Z` on the default branch | `release-*` — cross-platform build, zip packaging, GitLab Release |

After `bump-version` and pushing the tag:

```bash
git push origin HEAD
git push origin v0.1.0
```

Release assets are attached to the **GitLab Release** for the tag (and available as pipeline
artifacts under `dist/`).

### GitLab runner notes

- `release-windows` and `release-macos` use GitLab.com shared runner tags
  (`saas-windows-medium-amd64`, `saas-macos-medium-m1`). Change the `tags` in `.gitlab-ci.yml`
  for self-hosted or other GitLab instances.
- If you lack Windows/macOS runners, build those binaries elsewhere and use
  `package-godot-release.sh` locally, or adjust the pipeline to match your infrastructure.

## If using another git host

Keep using `ci-check` locally and in your own CI. Copy the commands from `rust-check` /
`.github/workflows/rust.yml`. For releases, run `bump-version` and
`scripts/linux/package-godot-release.sh` with binaries you build on each target platform.
