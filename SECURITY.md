# Security Policy

## Supported Versions

Pre-1.0. Only `0.1.x` is supported. Security fixes go into the
latest minor.

| Version | Supported |
| --- | --- |
| 0.1.x   | yes |
| < 0.1   | no  |

## Reporting a Vulnerability

Do not open a public issue for security vulnerabilities.

Email [ahmedayobbusiness@gmail.com](mailto:ahmedayobbusiness@gmail.com)
with:

- a description of the issue
- the affected version (`cargo pkgid` or `pnpm list @gentleduck/md`)
- a minimal reproducer if possible
- your assessment of the impact

We aim to acknowledge within 72 hours and release a fix or mitigation
within 30 days for high severity issues.

## Threat surfaces

dmc compiles content authored by repo contributors. The relevant
attack surfaces:

- **Raw HTML in MDX**: dmc passes raw `<div>` blocks through
  unsanitised. Sanitise downstream (rehype-sanitize via the sidecar,
  or a server-side HTML sanitiser) if MDX comes from untrusted
  authors.
- **Sidecar Node process**: the optional `@gentleduck/md-sidecar`
  spawns a Node child to run JS plugins. The plugin code runs with
  the same privileges as the build process. Pin plugin versions and
  audit them like any other build dep.
- **Cache files**: `<output>/.cache/dmc/*.json` contain compiled
  output. Treat them as build artifacts; do not import from cache
  paths at runtime.
- **NAPI bindings**: `@gentleduck/md` ships a native `.node` binary.
  Use the published npm package; do not load arbitrary `.node` files.

For more depth see
[`dmc-docs/guides/security.md`](dmc-docs/guides/security.md).
