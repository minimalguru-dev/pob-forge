# Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io/), certificate by
[SignPath Foundation](https://signpath.org/).

PoE Forge's Windows installer and the program inside it are signed with that certificate once SignPath Foundation
has approved the project; releases before that are unsigned. Only builds that the
[release workflow](.github/workflows/release.yml) produces on GitHub-hosted runners, from a tagged commit of this
repository, are submitted for signing; builds made anywhere else are not signed.

## Team and roles

| Role | Who |
| --- | --- |
| Committers and reviewers | [minimalguru-dev](https://github.com/minimalguru-dev) |
| Approvers | [minimalguru-dev](https://github.com/minimalguru-dev) |

Changes from anyone outside the team are reviewed by a team member before they are merged. Every signing request
is approved by hand in SignPath before anything is signed. Team members use multi-factor authentication on GitHub and
on SignPath.

## What is signed

- `pob-forge.exe`, the program, and `PoE Forge_<version>_x64-setup.exe`, its installer, for each release.
- Both carry the product name "PoE Forge" and the release's version, which SignPath checks before signing.
- The installer has an uninstaller (Windows Settings → Apps), and installing changes no system settings beyond
  the program's own files, shortcuts and its `pob:`/`pob2:` link handlers.

## Privacy

PoE Forge sends nothing to its developers: there is no telemetry, no analytics and no account with us.

It connects to other services as follows.

**On its own:**
- The app's file server, for passive tree art, item art and game data the installer does not carry.
- GitHub (or the app's file server's copy) to check for an update, in release builds.
- poe.ninja for price lists, when the overlay's Prices option is on. Whole lists are downloaded; the items you
  copy are not sent.

**Only when you ask for it:**
- Build sharing sites (pobb.in, pastebin, pob.codes, rentry, maxroll, mobalytics), when you import or share a build
  link.
- pathofexile.com, when you search the trade site, import a character or sign in to your account.
- poeprices.info, only after you turn on its estimate in Settings → Overlay. The text of the item you price-check
  is sent there.
- The AI provider you set up for the assistant, when you send it a message. What you write and the build summary
  go to that provider under its own terms.
- Wiki and database pages, which open in your own browser.

The overlay reads the clipboard once each time you press its shortcut, and reads the game's own log file. Neither is
sent anywhere except as listed above. See [OVERLAY.md](OVERLAY.md#what-it-reads-and-what-it-never-does).
