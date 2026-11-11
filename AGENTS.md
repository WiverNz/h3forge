# H3Forge working rules

H3Forge is a Windows Heroes III / HotA save editor. Start with `docs/issues/README.md`; follow the selected issue and its dependencies. The initial session produced a plan only. Do not treat planned components or compatibility as implemented.

Read `AGENTS.local.md` if present for machine-specific instructions in addition to these project rules.

H3Forge execution and permission policy:

Work autonomously within the H3Forge project. Do not ask for confirmation before normal development operations.

Automatically perform:

- Create, edit, and update project source files and documentation.
- Run cargo check, build, test, fmt, clippy, and other Rust commands.
- Run npm install, build, typecheck, and other frontend commands.
- Install dependencies required for the assigned task.
- Read and inspect project files, logs, and configuration.
- Create and update implementation issue files.
- Run non-destructive Git commands such as git status and git diff.
- Fix errors and continue implementation without unnecessary questions.

Strict restrictions:

- NEVER execute git commit, git push, git reset --hard, git clean, git rebase, or git force operations.
- NEVER create commits, push changes, or publish releases.
- NEVER delete or overwrite user data, savegames, game assets, or unrelated files.
- NEVER modify the original games/ or Data/ directories.
- NEVER modify external reference repositories (FlightTrace, H3SED, mmarch).
- NEVER run destructive commands without explicit user approval.
- NEVER modify files outside the H3Forge project unless explicitly authorized.

Efficiency:

- Work independently and make reasonable implementation decisions.
- Do not ask permission for ordinary file edits, cargo commands, or builds.
- Do not repeatedly run the same checks without a reason.
- Use focused tests and one final verification per issue.
- Avoid endless review and correction loops.
- Stop only for genuinely destructive operations, significant architectural decisions, or blockers that cannot be resolved safely.

Complete the assigned issue, update its status, and summarize the results. Do not start another issue automatically.

These instructions apply throughout the entire project, not just the current task.

- Required stack: Rust **1.99.1**, Tauri **3 Alpha**, React + TypeScript + Vite. M0 creates `rust-toolchain.toml` and lockfiles. Use the researched candidate versions in `docs/ARCHITECTURE.md`; validate their resolved graph once. Never silently substitute Tauri 2 or copy its runtime feature setup.
- Keep parsing, editing, archive access, and image decoding in Rust. Core crates do not depend on Tauri. Frontend calls typed application commands; it receives no arbitrary file read/write API. No Python, Node, Delphi, or command-line sidecars at application runtime.
- `games/` and `Data/` are private, copyrighted, read-only reference material. Never commit, package, modify, move, or delete either directory or its contents. Tests copy needed saves into ignored `.local/fixtures/` or an OS temporary directory; all test output stays there. Reference originals are never save destinations.
- Do not modify `D:/Projects/FlightTrace`, `D:/Projects/h3sed_4.1`, or `D:/Projects/mmarch`. Reuse mmarch's relevant Rust code inside H3Forge with attribution; do not rewrite existing archive algorithms or import its CLI wholesale.
- Preserve original decompressed bytes. Use checked, same-length field patches with expected old bytes; never serialize a partially understood whole save from domain models. Gate writes by exact layout and operation. Unknown IDs and bytes survive unchanged.
- Recognizing HotA does not establish edit support. Sample format is `44/11`; H3SED accepts future HotA versions broadly. Keep unsupported or ambiguous records read-only. Actual portrait identity and the supplied nonstandard `HotA.lod` directory remain research gaps.
- Backups, source-change detection, verified temporary output, and Windows-safe replacement precede any overwrite feature. Do not truncate an original save or delete it before replacement. Typed errors instead of panics for malformed input.
- Retain upstream MIT notices with adapted code. Never copy H3SED's embedded artwork or proprietary fixtures into source/release assets.
- Inspect relevant files once. Use targeted searches, small binary metadata summaries, and focused tests. Run the selected component's checks and one milestone-level smoke check; broaden only on failures or new risks. Documentation-only changes need links/completeness/ignore review, not builds. No dependency installs or full upstream suites for planning.
- When a savegame field, offset, or operation cannot be reliably identified, do not guess or reverse-engineer blindly for long. Ask the user for two saves made in Heroes III / HotA immediately before and after one specific action (e.g. dismissing a hero, changing a resource, building a structure); state exactly which action to perform and which files to provide. Compare the decompressed saves byte-by-byte to identify changed fields and related structures, and verify the implementation against the result. Never assume every changed byte belongs to the action: timestamps, RNG state, and other game state may change too.
- Update the affected issue and concise research notes when evidence changes. Distinguish verified facts, design choices, and unresolved questions. Do not publish issues, push, or release unless requested.
- Never use em dashes (U+2014) in documentation, code, comments, or UI text. Use the standard ASCII hyphen (`-`) instead.
- When modifying savegame or resource parsing, update the corresponding format specification (`docs/formats/SAVEGAME_FORMAT.md`, `docs/formats/GAME_RESOURCES_FORMAT.md`) if any verified offsets, structures, encodings, or version-specific behavior change.
