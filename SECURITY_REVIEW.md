# Progression preview security review — 2026-11-15

This is a scoped source/dependency review and hardening record, not a security certification or a completed penetration test. It covers Import/clipboard handling, art-cache writes, PoE OAuth, Codex launch/authentication, application credential namespacing, and the committed dependency versions. No real user password, token, session cookie or auth cache was read, and no account was authorized during the review.

## Account recommendation

- **PoE:** authorization is deliberately unavailable in the Progression preview, in both UI and backend. The fork must resolve its OAuth registration before enabling it. GGG documents one product per registered application and the public-client requirements for distributed executables; MIT licensing does not grant an OAuth application identity. Use code/XML/URL imports meanwhile. [GGG developer documentation](https://www.pathofexile.com/developer/docs)
- **OpenAI through Codex:** the preview launches the vendor CLI with its own `CODEX_HOME` and `cli_auth_credentials_store="keyring"`, including login. It removes inherited OpenAI/Codex API-key and OpenAI endpoint/organization/project overrides from that child process. Use the assistant's Sign in action for this profile. The credential-store setting deliberately fails if that store is unavailable; there is no requested plaintext fallback. Official Codex documentation describes these storage modes, and Codex derives the keyring identity from the profile path. [Codex authentication](https://learn.chatgpt.com/docs/auth), [Codex storage implementation](https://github.com/openai/codex/blob/main/codex-rs/login/src/auth/storage.rs)
- Application-managed credentials use the configured Tauri identifier, separating the preview from upstream. Existing upstream credentials are not migrated or deleted. Windows application-managed fallback storage remains DPAPI-protected in the application's own directory; this is distinct from Codex's required keyring mode.
- AI conversation and build data accessed by the assistant can leave the machine for the selected provider. A separate profile is not an OS security boundary against another process running as the same user. Other supported vendor CLIs have not received equivalent profile isolation in this change.

The previous `90cd6bb` installer predates this hardening. The new code is not evidence that an older installed executable has changed. Neither a successful compiler run nor source review proves that real-account login works on a particular machine; real login remains untested.

## Code changes and checks

- PoE token HTTP requests do not follow redirects; a synthetic local-server regression test verifies a 307 does not forward a form to its target.
- Tests inspect the Codex child launch for its separate profile, removed API-key variables and required keyring override. They do not use real credentials or contact OpenAI.
- Paste and Import now exposes empty/unavailable clipboard and import errors, with bounded retries after an explicit click. Browser tests exercise the actual Import view with IPC fixtures for code, URL and XML dispatch and failure recovery. They are not a native WebView clipboard certification.
- Atomic cache writes preserve the underlying OS error and use unique temporary filenames. Cleanup removes retired content-addressed images and leaves active temporary files and unrelated files intact. Regression tests cover file replacement, write failure and cleanup behavior.
- MCP retains the existing loopback binding and bearer authentication. Progression adds no public MCP tool, gameplay automation, game-memory reading or cookie collection.

## Dependency review

The review queried Bun's audit and OSV for 657 Cargo registry package versions. Public version metadata only was submitted. A genuine Cargo resolver update generated the lockfile in [CI](https://github.com/minimalguru-dev/pob-forge/actions/runs/43723959255), rather than hand-editing package checksums. Native validation uses `--locked`.

| Dependency | Patched version |
| --- | --- |
| undici | 7.30.1 |
| zod | 4.6.6 |
| rmcp | 3.9.0 |
| rustix | 1.1.6 |
| rustls-webpki | 0.103.16 |
| webpki-root-certs | 1.0.10 |

The audit is **not clean**. Remaining findings are tracked rather than suppressed:

- `deepmerge 4.3.1`, low severity, [GHSA-gcqx-w48m-w2h4](https://github.com/advisories/GHSA-gcqx-w48m-w2h4), no published patched version at review time. It is in the frontend build toolchain, not the imported-build parser. Build only trusted project configuration; do not interpret this as a finding in the PoB Lua calculation engine.
- `glib 0.18.5`, [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html), also reported under GHSA-wrw7-89jp-8q8g. This belongs to the Linux GTK dependency stack. This review does not certify the Linux build for arbitrary untrusted data.
- Maintenance notices: `proc-macro-error 1.0.4` ([RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html)); `zbus_names 4.3.4`, `zvariant 5.15.0`, `zvariant_derive 5.15.0` (RUSTSEC-2026-0371, RUSTSEC-2026-0370, RUSTSEC-2026-0372). The latter are D-Bus-related packages. A maintenance notice is not itself evidence of exploitable behavior in this Windows preview. Platform reachability was not independently established with a complete target-specific dependency audit.

## Validation limits and handoff

See DEVELOPMENT_LOG.md for exact tested revisions and CI results. Local frontend checks passed: 28 unit tests, six browser tests, Svelte zero errors/warnings and production bundle generation. Native test/build results must be recorded separately.

Desktop automation was unavailable due to a tool startup error. Automatic approval review also rejected an attempted native-app launch with a loopback debugging port, without a detailed reason; it was not retried or bypassed. Interactive desktop behavior and real account login remain unverified. The private upstream golden corpus is unavailable. The preview installer is unsigned and is not a production release.

For follow-up, preserve the disabled PoE guard until registration is resolved; validate Codex login with a deliberate user-driven sign-in; update remaining dependencies through their owning platform/toolchain rather than removing warnings or weakening credential storage. Follow SECURITY.md for any newly discovered vulnerability report.
