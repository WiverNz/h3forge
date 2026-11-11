//! Map text metadata (name, description) parsing.
//!
//! Ported from h3sed `Savefile.parse_metadata` (metadata.py, MIT copyright
//! 2020 Erki Suurjaak; see licenses/h3sed-MIT.txt). The h3sed header regex
//! comment claims big-endian text lengths; the actual parser uses the
//! little-endian `bytoi`, which is what this port implements (research:
//! RESEARCH.md "Fields and offsets"). Lengths are read from the fields, and
//! all reads are bounded by the payload length.

/// Map text fields parsed from the save payload header.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MapMetadata {
    /// Raw map name bytes as stored.
    pub name_raw: Vec<u8>,
    /// Lossy display form of the name (see [`decode_legacy_text`]).
    pub name_lossy: String,
    /// Raw map description bytes as stored, if present and valid.
    pub description_raw: Option<Vec<u8>>,
    /// Lossy display form of the description.
    pub description_lossy: Option<String>,
    /// Map edge length in tiles: the `u32` directly before the name field
    /// (252 in the HotA 44/11 sample, equal to the source map's header).
    pub size: Option<u32>,
    /// Whether the map has an underground level: the byte between the size
    /// word and the name length (sample-verified against the source map).
    pub has_underground: Option<bool>,
    /// Map difficulty 0-4: the byte directly after the description
    /// (sample-verified against the source map). `None` without a
    /// description or when the byte is out of range.
    pub difficulty: Option<u8>,
}

impl MapMetadata {
    /// Whether any metadata was found; absence is a diagnostic, not an error.
    pub fn is_found(&self) -> bool {
        !self.name_raw.is_empty()
    }
}

/// Header window h3sed searches for the metadata fields.
const HEADER_WINDOW: usize = 2048;

/// h3sed `.{0,100}` gap between magic and the name length field.
const MAX_MAGIC_GAP: usize = 100;

/// Hard cap on a single metadata text, guarding the bounded reads.
const MAX_TEXT_LEN: usize = 1024;

/// Decodes legacy single-byte text for display. UTF-8 content survives;
/// anything else falls back to ISO-8859-1 byte identity (total, never
/// panics). Saves in legacy codepages (e.g. CP1251) display as mojibake:
/// the raw bytes are preserved and selectable decoding is a frontend concern.
pub fn decode_legacy_text(raw: &[u8]) -> String {
    match std::str::from_utf8(raw) {
        Ok(text) => text.to_string(),
        Err(_) => raw
            .iter()
            .map(|&byte| char::from_u32(byte as u32).expect("u8 maps to char"))
            .collect(),
    }
}

/// Parses map name and description from the payload header.
///
/// Mirrors h3sed's greedy search: the largest gap `k` in 0..=100 after the
/// magic such that `raw[q] != 0`, `raw[q+1..q+4] == 0`, and consecutive
/// little-endian length-prefixed non-NUL texts follow at `q + 5`. Field
/// lengths always come from the stored length fields, never from the scan.
pub fn parse_map_metadata(payload: &[u8]) -> MapMetadata {
    let window = payload.len().min(HEADER_WINDOW);
    let raw = &payload[..window];
    let mut metadata = MapMetadata::default();
    let Some(k) = find_name_field_offset(raw) else {
        return metadata;
    };
    let mut pos = k + 5;
    let Some(name_raw) = read_text_field(payload, &mut pos) else {
        return metadata;
    };
    metadata.name_lossy = decode_legacy_text(&name_raw);
    metadata.name_raw = name_raw;
    // The locator guarantees raw[k] != 0 and raw[k+1..k+4] == 0: a small
    // little-endian u32, followed by one flag byte.
    metadata.size = Some(raw[k] as u32);
    metadata.has_underground = match raw[k + 4] {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    };
    if let Some(description_raw) = read_text_field(payload, &mut pos) {
        metadata.description_lossy = Some(decode_legacy_text(&description_raw));
        metadata.description_raw = Some(description_raw);
        metadata.difficulty = payload.get(pos).copied().filter(|&byte| byte <= 4);
    }
    metadata
}

/// Finds the offset of the 2-byte name length field, replicating h3sed's
/// greedy `.{0,100}` preference (largest gap first).
fn find_name_field_offset(raw: &[u8]) -> Option<usize> {
    for k in (0..=MAX_MAGIC_GAP).rev() {
        let q = k.checked_add(5)?; // magic length 5; keeps q + 5 <= window checked below
        if q + 5 > raw.len() {
            continue;
        }
        if raw[q] == 0 || raw[q + 1] != 0 || raw[q + 2] != 0 || raw[q + 3] != 0 {
            continue;
        }
        // Validate that length-prefixed non-NUL text actually follows.
        let field = q + 5;
        let Some(len) = text_len_at(raw, field) else {
            continue;
        };
        let content = field + 2;
        if len == 0 || content + len > raw.len() {
            continue;
        }
        if raw[content..content + len].contains(&0) {
            continue;
        }
        return Some(q);
    }
    None
}

/// Reads the 2-byte little-endian text length at `pos`.
fn text_len_at(raw: &[u8], pos: usize) -> Option<usize> {
    let bytes = raw.get(pos..pos + 2)?;
    Some(u16::from_le_bytes(bytes.try_into().ok()?) as usize)
}

/// Reads one little-endian length-prefixed text field starting at `*pos`,
/// advancing `*pos` past it. Returns `None` on any inconsistency; parsing of
/// further fields then stops, matching h3sed's per-field try/except.
fn read_text_field(payload: &[u8], pos: &mut usize) -> Option<Vec<u8>> {
    let len = text_len_at(payload, *pos)?;
    if len == 0 || len > MAX_TEXT_LEN {
        return None;
    }
    let content_start = *pos + 2;
    let content_end = content_start.checked_add(len)?;
    let raw = payload.get(content_start..content_end)?;
    if raw.contains(&0) {
        return None;
    }
    *pos = content_end;
    Some(raw.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_header_payload(name: &[u8], description: &[u8]) -> Vec<u8> {
        // H3SVG + 3 unknown bytes + nonzero + 3 zeros + 1 unknown byte, then
        // length-prefixed texts, matching the observed sample header shape.
        let mut payload = b"H3SVG".to_vec();
        payload.extend_from_slice(&[0, 0, 0]);
        payload.extend_from_slice(&44u32.to_le_bytes());
        payload.extend_from_slice(&11u32.to_le_bytes());
        payload.push(1); // nonzero at q
        payload.extend_from_slice(&[0, 0, 0]);
        payload.push(7); // skipped byte before name length
        payload.extend_from_slice(&(name.len() as u16).to_le_bytes());
        payload.extend_from_slice(name);
        payload.extend_from_slice(&(description.len() as u16).to_le_bytes());
        payload.extend_from_slice(description);
        payload.resize(1024, 0);
        payload
    }

    #[test]
    fn parses_name_and_description() {
        let payload = sample_header_payload(b"- The Mysterious Island - COOP", b"A map");
        let metadata = parse_map_metadata(&payload);
        assert!(metadata.is_found());
        assert_eq!(metadata.name_raw, b"- The Mysterious Island - COOP");
        assert_eq!(metadata.description_raw.as_deref(), Some(&b"A map"[..]));
        // The synthetic header stores 1 as the size word and 7 as the flag.
        assert_eq!(metadata.size, Some(1));
        assert_eq!(metadata.has_underground, None);
        assert_eq!(metadata.difficulty, Some(0));
    }

    #[test]
    fn parses_name_without_description() {
        let payload = sample_header_payload(b"My Map", b"");
        let metadata = parse_map_metadata(&payload);
        assert!(metadata.is_found());
        assert_eq!(metadata.name_raw, b"My Map");
        assert_eq!(metadata.description_raw, None);
    }

    #[test]
    fn missing_metadata_is_not_an_error() {
        let payload = vec![0u8; 2048];
        let metadata = parse_map_metadata(&payload);
        assert!(!metadata.is_found());
    }

    #[test]
    fn short_payload_is_safe() {
        let metadata = parse_map_metadata(b"H3SVG");
        assert!(!metadata.is_found());
        let metadata = parse_map_metadata(b"");
        assert!(!metadata.is_found());
    }

    #[test]
    fn nul_inside_text_rejects_field() {
        let payload = sample_header_payload(b"Bad\0Name", b"A map");
        let metadata = parse_map_metadata(&payload);
        // The name field fails validation, so the scan finds no metadata.
        assert!(!metadata.is_found());
    }

    #[test]
    fn decodes_utf8_and_legacy_bytes() {
        assert_eq!(decode_legacy_text("Héro".as_bytes()), "Héro");
        // High bytes map through ISO-8859-1 identity per byte value.
        assert_eq!(decode_legacy_text(&[0x41, 0x80]), "A\u{0080}");
    }
}
