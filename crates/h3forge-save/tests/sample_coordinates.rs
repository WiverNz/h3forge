//! Private movement evidence. Originals are copied before inspection.
use h3forge_save::{SaveDocument, apply_patches, compress_gzip};

fn fixtures() -> Option<Vec<SaveDocument>> {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping private coordinates fixtures: set H3FORGE_FIXTURE_ROOT");
        return None;
    };
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.local/fixtures/coordinates-test");
    std::fs::create_dir_all(&dir).unwrap();
    Some(
        ["before_movement", "moandor_left_1", "nimbus_water_step"]
            .iter()
            .map(|suffix| {
                let name = format!("SaAndVla_{suffix}.GM2");
                std::fs::copy(std::path::PathBuf::from(&root).join(&name), dir.join(&name))
                    .unwrap();
                SaveDocument::open(std::fs::read(dir.join(name)).unwrap()).unwrap()
            })
            .collect(),
    )
}

#[test]
fn native_movement_evidence() {
    let Some(docs) = fixtures() else { return };
    let expected = [
        ([231, 52, 0], [76, 41, 0]),
        ([230, 52, 0], [76, 41, 0]),
        ([231, 52, 0], [77, 42, 0]),
    ];
    for (doc, (moandor, nimbus)) in docs.iter().zip(expected) {
        assert_eq!(doc.hero_coordinates(66), Some(moandor));
        assert_eq!(doc.hero_coordinates(75), Some(nimbus));
        assert_eq!(doc.hero_coordinates(usize::MAX), None);
        let reopened = SaveDocument::open(compress_gzip(doc.payload())).unwrap();
        assert_eq!(reopened.payload(), doc.payload());
        assert_eq!(reopened.hero_coordinates(66), Some(moandor));
        assert_eq!(reopened.hero_coordinates(75), Some(nimbus));
    }
    let before = docs[0].payload();
    // Arbitrary-distance moves, not just reproductions of the native steps.
    for (id, to) in [(66, [108, 77, 0]), (75, [118, 71, 0])] {
        let patches = docs[0].plan_hero_relocation(id, to).unwrap();
        let mut payload = before.to_vec();
        apply_patches(&mut payload, &patches).unwrap();
        let reopened = SaveDocument::open(compress_gzip(&payload)).unwrap();
        assert_eq!(reopened.hero_coordinates(id), Some(to));
        // Return to the vacated tile verifies both occupancy directions and
        // the unchanged underlying state, including boarded-boat references.
        let back = reopened
            .plan_hero_relocation(id, docs[0].hero_coordinates(id).unwrap())
            .unwrap();
        apply_patches(&mut payload, &back).unwrap();
        let h = docs[0].heroes[id].span.start;
        let mut expected = before.to_vec();
        expected[h + 19..h + 27].fill(255);
        expected[h + 1069] = 1;
        assert_eq!(payload, expected);
    }
    for (id, to, native) in [(66, [230, 52, 0], 1), (75, [77, 42, 0], 2)] {
        let patches = docs[0].plan_hero_relocation(id, to).unwrap();
        let mut payload = before.to_vec();
        apply_patches(&mut payload, &patches).unwrap();
        // Every written byte agrees exactly with the native step. Normal-game
        // movement cost/facing/exploration/replay are deliberately preserved.
        for patch in &patches {
            assert_eq!(
                &payload[patch.offset..patch.end()],
                &docs[native].payload()[patch.offset..patch.end()]
            );
        }
        let remaining: Vec<_> = (0..4_921_359)
            .filter(|&i| payload[i] != docs[native].payload()[i])
            .collect();
        let incidental: Vec<_> = if id == 66 {
            (908..927)
                .chain([
                    4_277_472, 4_381_936, 4_381_956, 4_392_380, 4_392_384, 4_872_060,
                ])
                .collect()
        } else {
            (908..914)
                .chain(915..918)
                .chain(919..922)
                .chain(923..929)
                .chain([4_392_376, 4_392_396, 4_577_680, 4_872_060])
                .collect()
        };
        assert_eq!(remaining, incidental);
        assert_eq!(payload.len(), before.len());
        for i in 0..payload.len() {
            if !patches.iter().any(|p| (p.offset..p.end()).contains(&i)) {
                assert_eq!(payload[i], before[i]);
            }
        }
        let reopened = SaveDocument::open(compress_gzip(&payload)).unwrap();
        assert_eq!(reopened.hero_coordinates(id), Some(to));
        assert_eq!(
            reopened.heroes[id].movement_left,
            docs[0].heroes[id].movement_left
        );
        assert!(reopened.plan_hero_relocation(id, to).unwrap().is_empty());
        assert!(docs[0].plan_hero_relocation(id, [252, 52, 0]).is_err());
        assert!(docs[0].plan_hero_relocation(id, [to[0], to[1], 1]).is_err());
        assert!(
            docs[0]
                .plan_hero_relocation(
                    id,
                    docs[0]
                        .hero_coordinates(if id == 66 { 75 } else { 66 })
                        .unwrap()
                )
                .is_err()
        );
    }
    // Adversarial mutations stay on copies. Tile walking still succeeds, but
    // destination legality/linked-state gates must reject each altered save.
    for (at, value) in [
        (389_543 + 7, 3),      // inaccessible destination
        (389_543, 9),          // rock
        (389_543 + 7, 0x12),   // actionable/occupied
        (389_543 + 22, 255),   // inconsistent render reference
        (4_381_895 + 7, 0),    // coordinate duplicate disagrees
        (4_381_921 + 1069, 2), // unknown movement-state value
        (4_381_921 + 6, 1),    // adventure-spell state
    ] {
        let mut payload = before.to_vec();
        payload[at] = value;
        let malformed = SaveDocument::open(compress_gzip(&payload)).unwrap();
        assert!(
            malformed.plan_hero_relocation(66, [230, 52, 0]).is_err(),
            "accepted mutation at {at}"
        );
    }
    let mut payload = before.to_vec();
    payload[4_276_322 + 7] = 0;
    let no_boat = SaveDocument::open(compress_gzip(&payload)).unwrap();
    assert!(no_boat.plan_hero_relocation(75, [77, 42, 0]).is_err());
    // Exhaustive native diff, not a relocation recipe. The replay insertion
    // is aligned explicitly so shifted bytes cannot be mistaken for changes.
    let cases: [(usize, usize, &[(usize, usize)], &str); 2] = [
        (
            1,
            55,
            &[
                (908, 927),
                (389550, 389552),
                (389557, 389561),
                (389576, 389578),
                (389583, 389587),
                (4277472, 4277473),
                (4381895, 4381896),
                (4381902, 4381903),
                (4381936, 4381937),
                (4381940, 4381948),
                (4381956, 4381957),
                (4382990, 4382991),
                (4392380, 4392381),
                (4392384, 4392385),
                (4872060, 4872061),
            ],
            "0c0710000000080000004200000005000000060000000c070c00000009000000010000004200000001074200000006e7003400e6003400",
        ),
        (
            2,
            67,
            &[
                (908, 914),
                (915, 918),
                (919, 922),
                (923, 929),
                (308066, 308068),
                (308073, 308074),
                (314964, 314966),
                (314971, 314972),
                (4392335, 4392336),
                (4392337, 4392338),
                (4392342, 4392343),
                (4392344, 4392345),
                (4392376, 4392377),
                (4392380, 4392388),
                (4392396, 4392397),
                (4393430, 4393431),
                (4577680, 4577681),
                (4872060, 4872061),
            ],
            "0c0710000000080000004b00000000000000030000000c070c00000009000000010000004b00000001074b000000034c0029004d002a000b07010053002c007e00ff00",
        ),
    ];
    const INSERT: usize = 4_921_359;
    for (index, growth, ranges, replay_hex) in cases {
        let after = docs[index].payload();
        assert_eq!(after.len(), before.len() + growth);
        assert_eq!(&before[INSERT..], &after[INSERT + growth..]);
        let changed: Vec<_> = (0..INSERT).filter(|&i| before[i] != after[i]).collect();
        let expected: Vec<_> = ranges.iter().flat_map(|&(s, e)| s..e).collect();
        assert_eq!(changed, expected);
        let replay: Vec<u8> = (0..replay_hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&replay_hex[i..i + 2], 16).unwrap())
            .collect();
        assert_eq!(&after[INSERT..INSERT + growth], replay);
        // All templates/objects, boats, town links and hero owner tables stay
        // byte-identical in these two samples. Not proof for other movements.
        assert_eq!(&before[3_550_135..4_276_427], &after[3_550_135..4_276_427]);
        for town in &docs[0].towns {
            assert_eq!(&before[town.span.clone()], &after[town.span.clone()]);
        }
        assert_eq!(&before[4_554_778..4_555_208], &after[4_554_778..4_555_208]);
    }
}
