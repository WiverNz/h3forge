//! Opt-in copy-based 44/11 resource regression, never writes a game original.
use h3forge_save::{PlayerEdit, SaveDocument};

#[test]
fn sample_players_exact_patch_roundtrip_and_gates() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping private player fixture: set H3FORGE_FIXTURE_ROOT");
        return;
    };
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.local/fixtures/players");
    std::fs::create_dir_all(&dir).unwrap();
    let source = std::fs::read(std::path::PathBuf::from(root).join("SaAndVla.GM2")).unwrap();
    std::fs::write(dir.join("source.GM2"), &source).unwrap();
    let doc = SaveDocument::open(std::fs::read(dir.join("source.GM2")).unwrap()).unwrap();
    assert_eq!(
        doc.players.len(),
        8,
        "town anchor: {:?}",
        doc.towns
            .iter()
            .take(2)
            .map(|t| (&t.span, t.confidence))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        doc.players
            .iter()
            .filter(|p| p.active)
            .map(|p| p.color)
            .collect::<Vec<_>>(),
        [0, 4, 6, 7]
    );
    let expected = [
        [735, 1607, 920, 564, 123, 114, 8721404],
        [149, 14, 296, 42, 190, 47, 52210],
        [33, 26, 39, 26, 32, 29, 11262],
        [40, 26, 40, 29, 26, 29, 11132],
        [382, 66, 352, 48, 85, 13, 552235],
        [82, 20, 34, 19, 24, 16, 25355],
        [760, 285, 1009, 376, 248, 125, 89816],
        [190, 59, 280, 256, 6, 5, 61676],
    ];
    for (p, values) in doc.players.iter().zip(expected) {
        assert_eq!(p.resources, values);
        assert!(p.read_only_reason.is_none(), "{p:?}");
        assert_eq!(p.status.code(), if p.active { "active" } else { "unknown" });
    }
    for player in 0..8 {
        for resource in 0..7 {
            let edit = PlayerEdit::SetResource {
                resource,
                value: 1234567,
            };
            let patches = doc.plan_player_edits(player, &[edit]).unwrap();
            assert_eq!(patches.len(), 1);
            assert_eq!(
                patches[0].offset,
                4634994 + player * 149 + 98 + resource as usize * 4
            );
            assert_eq!(patches[0].expected.len(), 4);
            let reopened =
                SaveDocument::open(doc.serialize_with_patches(&patches).unwrap()).unwrap();
            assert_eq!(
                reopened.players[player].resources[resource as usize],
                1234567
            );
            assert_eq!(reopened.players[player].status, doc.players[player].status);
            let at = patches[0].offset;
            assert_eq!(&doc.payload()[..at], &reopened.payload()[..at]);
            assert_eq!(&doc.payload()[at + 4..], &reopened.payload()[at + 4..]);
        }
    }
    let mut patches = doc
        .plan_player_edits(
            4,
            &[PlayerEdit::SetResource {
                resource: 6,
                value: 2147483647,
            }],
        )
        .unwrap();
    patches.extend(
        doc.plan_player_edits(
            6,
            &[PlayerEdit::SetResource {
                resource: 0,
                value: 0,
            }],
        )
        .unwrap(),
    );
    let output = doc.serialize_with_patches(&patches).unwrap();
    std::fs::write(dir.join("edited.GM2"), output).unwrap();
    let reopened = SaveDocument::open(std::fs::read(dir.join("edited.GM2")).unwrap()).unwrap();
    let changed: Vec<_> = doc
        .payload()
        .iter()
        .zip(reopened.payload())
        .enumerate()
        .filter_map(|(i, (a, b))| (a != b).then_some(i))
        .collect();
    assert!(
        changed
            .iter()
            .all(|&i| patches.iter().any(|p| (p.offset..p.end()).contains(&i)))
    );
    assert_eq!(reopened.players[4].resources[6], i32::MAX);
    assert_eq!(reopened.players[6].resources[0], 0);
    println!(
        "two-player diff: {changed:?}; unchanged bytes: {}",
        doc.payload().len() - changed.len()
    );
    for p in [1, 2, 3, 5] {
        assert_eq!(doc.players[p], reopened.players[p]);
    }
    for value in [-1, 2147483648, i64::MAX] {
        assert!(
            doc.plan_player_edits(0, &[PlayerEdit::SetResource { resource: 6, value }])
                .is_err()
        );
    }
    assert!(
        doc.plan_player_edits(
            0,
            &[PlayerEdit::SetResource {
                resource: 7,
                value: 1
            }]
        )
        .is_err()
    );
    assert!(
        doc.plan_player_edits(
            8,
            &[PlayerEdit::SetResource {
                resource: 6,
                value: 1
            }]
        )
        .is_err()
    );
    let edit = PlayerEdit::SetResource {
        resource: 6,
        value: 1,
    };
    assert!(doc.plan_player_edits(0, &[edit.clone(), edit]).is_err());
    assert!(
        doc.plan_player_edits(
            0,
            &[PlayerEdit::SetResource {
                resource: 6,
                value: 8721404
            }]
        )
        .unwrap()
        .is_empty()
    );
    for offset in [4634994, 4634994 + 3, 4635590 + 26] {
        let mut payload = doc.payload().to_vec();
        payload[offset] = 254;
        let bad = SaveDocument::open(h3forge_save::compress_gzip(&payload)).unwrap();
        let id = if offset == 4635590 + 26 { 4 } else { 0 };
        if let Some(player) = bad.players.get(id) {
            assert_eq!(player.status.code(), "unknown");
            assert!(!player.active);
        }
        assert!(
            bad.plan_player_edits(
                id,
                &[PlayerEdit::SetResource {
                    resource: 6,
                    value: 1
                }]
            )
            .is_err()
        );
    }
    // A no-town countdown is not a historical participation/defeat flag.
    for days_left in [0, 7, 255] {
        let mut payload = doc.payload().to_vec();
        payload[4635143 + 23] = days_left;
        let changed = SaveDocument::open(h3forge_save::compress_gzip(&payload)).unwrap();
        assert_eq!(changed.players[1].status.code(), "unknown");
        assert!(changed.players[1].read_only_reason.is_none());
    }
    let mut unsupported = doc.payload().to_vec();
    unsupported[12..16].copy_from_slice(&12u32.to_le_bytes());
    let unsupported = SaveDocument::open(h3forge_save::compress_gzip(&unsupported)).unwrap();
    assert!(unsupported.players.is_empty());
    assert!(
        unsupported
            .plan_player_edits(
                0,
                &[PlayerEdit::SetResource {
                    resource: 6,
                    value: 1
                }]
            )
            .is_err()
    );
    let mut negative = doc.payload().to_vec();
    negative[4635092..4635096].copy_from_slice(&(-1i32).to_le_bytes());
    let negative = SaveDocument::open(h3forge_save::compress_gzip(&negative)).unwrap();
    assert_eq!(negative.players[0].resources[0], -1);
    assert!(
        negative
            .plan_player_edits(
                0,
                &[PlayerEdit::SetResource {
                    resource: 6,
                    value: 1
                }]
            )
            .is_err()
    );
    let patch = doc
        .plan_player_edits(
            0,
            &[PlayerEdit::SetResource {
                resource: 6,
                value: 1,
            }],
        )
        .unwrap()
        .remove(0);
    let mut payload = doc.payload().to_vec();
    assert!(h3forge_save::apply_patches(&mut payload, &[patch.clone(), patch.clone()]).is_err());
    assert_eq!(payload, doc.payload());
    let mut stale = patch;
    stale.expected[0] ^= 1;
    assert!(h3forge_save::apply_patches(&mut payload, &[stale]).is_err());
    assert_eq!(payload, doc.payload());
}
