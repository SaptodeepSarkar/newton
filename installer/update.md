# Updating Newton

Newton has no auto-updater and no update check. A new version is a file you
download and run, once, when you decide to. This page is that procedure.

## Why there is no auto-updater

An update check is a request. It leaves your machine before you asked for
anything, names you to a server, and proves the app talks to the network on its
own. [[PRD]] section 8 promises zero requests outside the Google allowlist and
"no update pings without a user action"; [[Security Audit]] finding 8 says one
update check makes that claim false without anyone noticing. [[Build Plan]]
phase 27 is this manual channel, and `scripts/no-phone-home.sh` fails the build
if a binary can even name another host.

So the honest answer is a download link, not a poll. Anything that checks for a
version needs a host to check against, and a host is the thing we are not
building.

## Updating

1. Open the project's Releases page and read the notes for the version you are
   installing. Note the version number.
2. Download the package for your platform and its checksum file, e.g.
   `newton-<version>-Linux-x86_64.tar.gz` and `SHA256SUMS`.
3. Verify the checksum before running anything.
4. Quit Newton, replace the program, start it again.
5. Delete the download.

The packages are unsigned, so the checksum is the only proof the file is the one
that was published. A checksum from the same page as the file proves the
download is intact, not who built it; only a signature would tell you that, and
these artifacts have none. See `installer/README.md` for what each package
contains.

### Verify the checksum

Linux and macOS:

```sh
sha256sum -c SHA256SUMS --ignore-missing
# macOS: shasum -a 256 -c SHA256SUMS --ignore-missing
```

On macOS with the disk image, substitute the `.dmg` name. If the output does not
end in `OK`, delete the file and download it again. Do not run a package whose
checksum did not match, even if the app launches.

Windows (PowerShell):

```powershell
Get-FileHash .\newton-<version>-Windows-x86_64.zip -Algorithm SHA256
```

Compare the printed hash against the line for that file in `SHA256SUMS`,
character for character. Windows gives no error on a mismatch; you have to look.

### Replace the program

| Platform | What to do |
|---|---|
| Linux `.tar.gz` | Unpack the new archive over the old directory. Keep the old `bin/` until the new one starts once. |
| Linux `.deb` | `sudo apt install ./newton-<version>-Linux-x86_64.deb`; it upgrades `/opt/newton` in place. |
| macOS | Open the `.dmg`, drag `Newton.app` to Applications, and choose Replace when asked. |
| Windows `.zip` | Unpack over the install directory with Newton closed. |

Keep the previous package until the new version has started and answered one
turn. Rolling back is reinstalling the old file, nothing more: there is no
downgrade path to break because there is no migration step.

## Your memory graph survives an update

Removing or replacing the program must never touch your data. The memory graph,
the embedding index and your Google tokens live outside the installed program:
tokens are in the OS keychain, and the graph is a local file in the app's user
data directory, not in the install directory. An update replaces the program and
nothing else.

- Do not "clean install" by deleting the user data directory. That deletes the
  memory graph, which is the part of Newton that cannot be rebuilt.
- If a release ever needs to change the graph format, its notes say so and the
  tool migrates the file on first start after the update. A release that cannot
  migrate a graph will say that too, before you install it, so you can copy the
  graph somewhere safe first.
- Back up the graph by copying that file while Newton is closed. There is no
  export in v1.

Checked 2026-11-01.
