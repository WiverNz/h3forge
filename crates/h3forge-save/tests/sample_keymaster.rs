//! Opt-in real pair and all 64 independently addressed bits, on private copies.
use h3forge_save::{SaveDocument, TentEdit, compress_gzip};
#[test]
fn native_pair_and_independent_bits_roundtrip() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping keymaster pair: set H3FORGE_FIXTURE_ROOT");
        return;
    };
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.local/fixtures/keymaster-core");
    std::fs::create_dir_all(&dir).unwrap();
    let mut sources = Vec::new();
    let mut docs = Vec::new();
    for phase in ["before", "after"] {
        let name = format!("SaAndVla_{phase}_visit_red_tent.GM2");
        let source = std::path::PathBuf::from(&root).join(&name);
        let bytes = std::fs::read(&source).unwrap();
        std::fs::write(dir.join(&name), &bytes).unwrap();
        docs.push(SaveDocument::open(std::fs::read(dir.join(&name)).unwrap()).unwrap());
        sources.push((source, bytes));
    }
    let a = &docs[0];
    let b = &docs[1];
    assert_eq!(a.keymaster_masks(), Some([64, 8, 1, 2, 4, 32, 126, 16]));
    assert_eq!(b.keymaster_masks(), Some([64, 8, 129, 2, 4, 32, 126, 16]));
    assert_eq!((a.payload().len(), b.payload().len()), (5012796, 5012847));
    assert_eq!(&a.payload()[4921809..], &b.payload()[4921860..]);
    let patch = a
        .plan_tent_edits(&[TentEdit {
            player: 7,
            color: 2,
            visited: true,
        }])
        .unwrap();
    assert_eq!(patch.len(), 1);
    assert_eq!(
        (
            patch[0].offset,
            patch[0].expected.to_vec(),
            patch[0].replacement.to_vec()
        ),
        (4552404, vec![1], vec![129])
    );
    let mut expected = a.payload().to_vec();
    expected[4552404] = 129;
    assert_eq!(
        SaveDocument::open(a.serialize_with_patches(&patch).unwrap())
            .unwrap()
            .payload(),
        expected
    );
    for player in 0..8 {
        for color in 0..8 {
            let before = a.keymaster_masks().unwrap();
            let edit = TentEdit {
                player,
                color,
                visited: before[color as usize] & (1 << player) == 0,
            };
            let patches = a.plan_tent_edits(&[edit]).unwrap();
            assert_eq!(patches.len(), 1);
            let reopened = SaveDocument::open(a.serialize_with_patches(&patches).unwrap()).unwrap();
            let changes: Vec<_> = a
                .payload()
                .iter()
                .zip(reopened.payload())
                .enumerate()
                .filter_map(|(i, (x, y))| (x != y).then_some(i))
                .collect();
            assert_eq!(changes, [4552402 + color as usize]);
            let mut expected = before;
            expected[color as usize] ^= 1 << player;
            assert_eq!(reopened.keymaster_masks(), Some(expected));
            assert!(reopened.plan_tent_edits(&[edit]).unwrap().is_empty());
            let reverse = TentEdit {
                visited: !edit.visited,
                ..edit
            };
            let restored = SaveDocument::open(
                reopened
                    .serialize_with_patches(&reopened.plan_tent_edits(&[reverse]).unwrap())
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(restored.payload(), a.payload());
        }
    }
    let edits = [
        TentEdit {
            player: 0,
            color: 2,
            visited: false,
        },
        TentEdit {
            player: 7,
            color: 2,
            visited: true,
        },
    ];
    let patches = a.plan_tent_edits(&edits).unwrap();
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].replacement.as_ref(), &[128]);
    assert!(a.plan_tent_edits(&[edits[0], edits[0]]).is_err());
    for edit in [
        TentEdit {
            player: 8,
            ..edits[0]
        },
        TentEdit {
            color: 8,
            ..edits[0]
        },
    ] {
        assert!(a.plan_tent_edits(&[edit]).is_err());
    }
    // Version and corrupted owner-table gates cannot be bypassed by public views.
    for (offset, value) in [(12, 12), (4551863 + 70, 0)] {
        let mut bytes = a.payload().to_vec();
        bytes[offset] = value;
        let bad = SaveDocument::open(compress_gzip(&bytes)).unwrap();
        assert!(bad.keymaster_masks().is_none());
        assert!(bad.plan_tent_edits(&edits).is_err());
    }
    for (path, bytes) in sources {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}
