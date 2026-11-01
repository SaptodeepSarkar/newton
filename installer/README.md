# Newton packages

These are development packages for Newton 0.1.0, built by CI from the current
commit. They are not a release, and they are not signed.

## What each package is

| Platform | File | What it is |
|---|---|---|
| Linux | `newton-0.1.0-Linux-<arch>.tar.gz` | Plain archive. Unpack and run `bin/newton`. |
| Linux | `newton-0.1.0-Linux-<arch>.deb` | Debian package, self-contained under `/opt/newton`. `sudo apt install ./newton-0.1.0-Linux-<arch>.deb`; run `/opt/newton/bin/newton`. |
| macOS | `newton-0.1.0-Darwin-<arch>.dmg` | Disk image containing `Newton.app`. Drag it to Applications, or run it from the image. |
| Windows | `newton-0.1.0-Windows-<arch>.zip` | Plain archive. Unpack and run `newton.exe`. |

The archive or disk image contains the executable, the QML module and the Qt
runtime it needs. A machine without Qt installed should be able to run it.

## These artifacts are unsigned

No package here has a code-signing certificate or a notarisation ticket.

- **macOS:** Gatekeeper will refuse to open the app normally. Only open it if
  you built it yourself or trust the source; then right-click the app and choose
  Open, or remove the quarantine flag with `xattr -dr com.apple.quarantine
  Newton.app`. Neither step makes the app trustworthy, it only tells macOS that
  you accepted the risk.
- **Windows:** SmartScreen will show "Windows protected your PC". You would have
  to choose "More info" and "Run anyway". Do not do that for an artifact whose
  origin you cannot verify.
- **Linux:** `.deb` files are unsigned, so `apt` cannot check their origin. The
  `.tar.gz` has no signature either.

Unsigned means you cannot tell from the file alone who built it or whether it
was changed after it was built. Verify it against the checksum or CI run it came
from before running it.

## Models are not included

No speech, language or wake-word model is in these packages. The model manifest
and downloader arrive in a later phase; until then the app starts but cannot
listen or answer. Models are deliberately kept out of git and out of CI
artifacts because they are large and separately licensed. When the downloader
lands, it will fetch them after installation, with a SHA-256 check per file.

## Signing

An unsigned build does not fail, it asks the user to override their own
operating system. On Windows the exe or installer triggers SmartScreen:
"Windows protected your PC", hidden behind "More info" and "Run anyway". On
macOS Gatekeeper refuses the first open with "cannot be opened because the
developer cannot be verified"; the user must right-click, Open, and confirm.
Both dialogs look like the artifact is damaged or malicious, and neither can be
suppressed after the fact. Sign before publishing:

| Platform | What is needed | Signing key |
|---|---|---|
| Linux | No signature is required for the `.deb`/`.tgz`; publish a SHA-256 next to it | Optional: an OpenPGP key for the repository `Release` file |
| macOS | `codesign` with a Developer ID Application certificate, plus a notarisation ticket | Apple Developer ID certificate (`.p12`) and App Store Connect API key or `notarytool` credentials |
| Windows | Authenticode signature from a code-signing certificate, timestamped | Code-signing certificate (`.pfx`/`.p12`) and its password |

Store each key as a CI secret and keep it out of logs and artifacts:

- Never `echo` a key, pass it on the command line where it lands in the process
  list, or leave it in a workspace that CI uploads.
- Never commit a `.p12`, `.pfx`, `.pem` or `*.key`; `.gitignore` them.
- Pipe secrets through stdin or a file mode `600` that is deleted after use, and
  check the build log for leaked material before sharing it.

`sign-check.sh <package> <linux|macos|windows>` verifies a built package and
exits 0 only when it is publishable. It never signs, and it reports a signature
as unverified rather than valid when `codesign`/`osslsigncode` is not installed.
