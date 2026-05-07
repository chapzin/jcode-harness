# Security Policy

## Supported versions

Security fixes are applied to the default branch of `chapzin/jcode-harness`. Until formal releases are cut, consumers should track the latest published `master` commit.

## Reporting a vulnerability

Please report vulnerabilities privately through GitHub Security Advisories or by opening a minimal issue that does not include secrets, tokens, private keys, or exploit details. Include affected commit/version, impact, reproduction steps, and whether the issue is already public.

## Secret handling

- Do not commit credentials, OAuth client secrets, API keys, private keys, `.env` contents, or personal access tokens.
- Google OAuth client IDs and client secrets must be supplied through user-controlled environment variables or local credential files. The project must not embed shared Google OAuth client credentials in source code, docs, or tests.
- `scripts/security_preflight.sh` scans tracked source/docs/scripts for common secret patterns, including Google OAuth client IDs/secrets.
- Auth/token files are written through hardened storage helpers where available.

## Code scanning triage policy

CodeQL findings that expose real secret material or missing least-privilege controls should be fixed in code. Findings that are verified false positives, such as test-only assert messages, user-facing CLI output, or HTTP request-builder flows that never log request headers/bodies, may be dismissed in GitHub with an explicit rationale after review.
