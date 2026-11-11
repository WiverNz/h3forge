# H3F-016 - Polish save browsing and the desktop editing experience

Status: planned. The separately requested compact layout and presentation follow-up is completed in [H3F-020](020-desktop-ui-usability.md); the broader scope below remains planned. Milestone: M5. Priority: P1.

## Goal

Make the completed MVP comfortable for everyday Windows use.

## Scope

Add selected-installation Games discovery, recent files, native drag/drop grants, search and owner/status filters. Finish consistent typography, spacing, dark/gold tokens, original-art presentation, keyboard focus and 100–200% DPI behavior. Unify dirty field/hero/save states and Save/Discard/Cancel on close/open. Persist small settings in the app-data directory. Keep unsupported versions/assets clearly explained.

## Relevant files or components

resource discovery; app commands/session/settings; web pages/components/styles; user documentation.

## Dependencies

H3F-005, H3F-008, H3F-009 See the [issue index](README.md) and [architecture](../ARCHITECTURE.md). Implementation paths are planned until created.

## Acceptance criteria

- [ ] Picker, discovered save and native drop all use the same validated load path; arbitrary frontend paths are not accepted.
- [ ] Unsaved changes survive failed saves and Cancel; reset/discard updates both UI and backend revisions.
- [ ] No assets, empty lists, corrupt saves and unknown fields produce understandable useful screens.
- [ ] Key edit/save actions work with keyboard and at common Windows DPI scales.

## Minimal verification

One manual desktop flow covering discovery/drop, filter/select/edit, cancel-close, discard, missing assets and scaled layout. Add only focused state-transition tests where they protect real dirty-state behavior.

