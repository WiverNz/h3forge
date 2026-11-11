# H3Forge portable Windows preview

Download `H3Forge.exe` and optionally `H3Forge.exe.sha256`. This is a Windows x64
portable application with its React frontend, application icon and adapted-code
MIT notices embedded. There is no installer and no loose frontend directory.
Microsoft Edge WebView2 Runtime must already be installed. No Python, Node.js,
Rust, Delphi or mmarch runtime is required. Game archives/artwork are not included.
The executable is unsigned; Windows may display an unknown-publisher warning.

Place it beside Heroes III / HotA to discover that installation, or run it from
another folder and use the installation picker. Keep your own game installation.
Work on copied saves until the matching-game checks below are completed.

## Supported preview scope

Sample round trips verify gated HotA 44/11 hero attributes, learned spells,
armies, secondary skills, backpack and supported equipment operations; verified
town garrisons, a subset of buildings, Mage Guild spells and player resources are
also editable. Writes use checked patches, backups and parser reopen verification.
Recognition of another layout does not authorize editing it. Ambiguous records,
unknown IDs and unsupported operations remain read-only.

Map-position editing and map revelation are unfinished. Some town buildings,
equipment combinations, scroll/equipment coupling and resource encodings remain
unsupported. Some extended hero portrait assignments are inferred. HotA opaque-ID
stored/zlib archives work; LZMA does not. In-game loading of edited saves remains
unverified: a parser round trip is not proof of game compatibility.

## Before a public release

Clean Windows 10 and 11 launch/edit/save/reopen, both placement modes, icon
appearance, missing-WebView2 behavior, four-language visual/translation review,
and matching-game loading remain manual gates. H3F-016 acceptance and the complete
transitive dependency license inventory also need release review. This workflow
does not certify those checks. See `docs/RELEASE.md` in the tagged source for
recovery instructions, verification commands and maintainer checks.
