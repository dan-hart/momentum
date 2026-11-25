# Momentum 0.5.0 macOS TestFlight handoff

## Session Summary

- November 25, 2026 at 08:10 Central
- Native macOS TestFlight setup, build 500 delivery, and release candidate CI
- Branch `task/0.5.0`

## What Was Done

- Added a sandboxed universal Mac beta and a GitHub Actions TestFlight workflow.
- Fixed a duplicate app plist that Apple detected and an obsolete backup
  confirmation in the Linux translation regression.
- Uploaded macOS 0.5.0 (500) from Xcode with automatic signing. App Store
  Connect reports `VALID` and `IN_BETA_TESTING`. Internal Testers can access
  it, and the five approved What to Test bullets were verified by readback.

## Why It Was Done

- Mac testers can install and update the native beta through TestFlight.
- The beta stores tasks and credentials separately from the stable Mac app.
- The release checks catch the packaging problem and require the current
  recovery-copy wording.

## Files Changed

- [Progress](../../docs/PROGRESS.md), [features](../../docs/FEATURES.md),
  and [bugs](../../docs/BUGS.md) record the release and verification.
- The [release guide](../../docs/RELEASING.md#macos-testflight) records
  the local upload fallback and how to check the exact Mac build.

## Knowledge Base Actions

- No separate KB update is applicable. The project release guide holds
  the reusable upload and validation procedure.

## Verification and Safety

- GitHub Actions passed the [primary CI](https://github.com/dan-hart/momentum/actions/runs/45626933659),
  [macOS](https://github.com/dan-hart/momentum/actions/runs/45626933602),
  and [Android](https://github.com/dan-hart/momentum/actions/runs/45626934353)
  workflows at `243bae2`.
- Documentation publication uses `git diff --cached --check` and
  `git secrets --scan --cached`. Signing keys, credentials, and private
  signing artifacts are excluded from the documentation.

## Commit and Push

- Release code and fixes are at `243bae2`.
- Find the documentation commit with `git log -1 -- research/session-logs/2026-11-25-0810-dcp-session-log.md`.
- Documentation branch publication targets are `origin` and `NelsonGitea`.

## Follow-Ups

- Hosted uploads require the workflow on `main` and the dedicated App Store
  Connect team key in the `macos-testflight` GitHub Actions environment.
- Native installation and sandboxed backup export and restore require
  verification on an installed beta.
