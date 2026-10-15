# Security policy

## Supported versions

Only the latest release receives security fixes. Aye is pre-1.0; upgrade
to the newest [release](https://github.com/aye-lang/aye/releases) before
reporting, if you can.

## Reporting a vulnerability

Please do not open a public issue or Discussion for a vulnerability.
Report it privately through GitHub's private vulnerability reporting:
on the [Security tab](https://github.com/aye-lang/aye/security) choose
"Report a vulnerability", or go to
<https://github.com/aye-lang/aye/security/advisories/new>.

Include the `aye --version` output, your OS, and the smallest program or
input that shows the problem. We will acknowledge the report, work on a
fix in the private advisory, and publish the advisory with the fix.

In scope: the compiler, `aye lsp`, the release binaries and install
script, std, and the framework. A program that is memory-unsafe because
it uses the documented unsafe features (raw pointers, `extern`, `-fast`
mode without checks) is not a vulnerability; a compiler that emits
unsafe C for safe code, or an std function that corrupts memory on valid
input, is.
