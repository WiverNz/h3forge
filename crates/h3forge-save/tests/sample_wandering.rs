//! Native surviving-count evidence and checked edits of two independent groups.
use h3forge_save::{SaveDocument, apply_patches, compress_gzip, neutral::NeutralEdit};

fn open(name: &str) -> Option<SaveDocument> {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping wandering fixture: H3FORGE_FIXTURE_ROOT unset");
        return None;
    };
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.local/fixtures/wandering");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(std::path::PathBuf::from(root).join(name), dir.join(name)).unwrap();
    Some(SaveDocument::open(std::fs::read(dir.join(name)).unwrap()).unwrap())
}

#[test]
fn native_72_to_56_and_two_independent_type_quantity_edits() {
    let (Some(before), Some(after)) = (
        open("SaAndVla_before_defeat.GM2"),
        open("SaAndVla_after_56.GM2"),
    ) else {
        return;
    };
    let b = before.neutral_armies();
    let a = after.neutral_armies();
    assert_eq!(b.unavailable_reason, None);
    assert_eq!(a.unavailable_reason, None);
    let ghost = b.groups.iter().find(|g| g.id == 13926).unwrap();
    let survivor = a.groups.iter().find(|g| g.id == 13926).unwrap();
    println!(
        "ghost reason {:?}, choices {}",
        ghost.read_only_reason,
        ghost.allowed_creatures.len()
    );
    assert_eq!(ghost.read_only_reason, None);
    assert_eq!(ghost.army.unwrap()[0].count, 72);
    assert_eq!(survivor.army.unwrap()[0].count, 56);
    assert_eq!(ghost.creature_id, Some(60));
    let native = before
        .plan_neutral_edits(
            ghost.id,
            &[NeutralEdit {
                slot: 0,
                creature_id: Some(60),
                count: 56,
            }],
        )
        .unwrap();
    assert_eq!(native.len(), 1);
    assert_eq!(native[0].offset, 4947801);
    assert_eq!(&*native[0].expected, &72u32.to_le_bytes());
    assert_eq!(&*native[0].replacement, &after.payload()[4948104..4948108]);
    let second = b
        .groups
        .iter()
        .find(|g| {
            g.class == 54
                && g.id != ghost.id
                && g.read_only_reason.is_none()
                && g.allowed_creatures
                    .iter()
                    .any(|&id| Some(id) != g.creature_id)
        })
        .unwrap();
    let mut patches = Vec::new();
    let mut desired = Vec::new();
    for (i, g) in [ghost, second].iter().enumerate() {
        let id = *g
            .allowed_creatures
            .iter()
            .find(|&&id| Some(id) != g.creature_id)
            .unwrap();
        let count = 1000 + i as u32;
        let planned = before
            .plan_neutral_edits(
                g.id,
                &[NeutralEdit {
                    slot: 0,
                    creature_id: Some(id),
                    count,
                }],
            )
            .unwrap();
        assert_eq!(
            planned.iter().map(|p| p.expected.len()).collect::<Vec<_>>(),
            [2, 2, 4]
        );
        patches.extend(planned);
        desired.push((g.id, id, count));
    }
    let mut payload = before.payload().to_vec();
    apply_patches(&mut payload, &patches).unwrap();
    assert_eq!(payload.len(), before.payload().len());
    for (offset, (&old, &new)) in before.payload().iter().zip(&payload).enumerate() {
        let replacement = patches
            .iter()
            .find(|p| (p.offset..p.end()).contains(&offset));
        assert_eq!(
            new,
            replacement
                .map(|p| p.replacement[offset - p.offset])
                .unwrap_or(old)
        );
    }
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.local/fixtures/wandering");
    std::fs::write(dir.join("edited.GM2"), compress_gzip(&payload)).unwrap();
    let reopened = SaveDocument::open(std::fs::read(dir.join("edited.GM2")).unwrap()).unwrap();
    assert_eq!(reopened.payload(), payload);
    let groups = reopened.neutral_armies();
    assert_eq!(groups.unavailable_reason, None);
    for (object, id, count) in desired {
        let g = groups.groups.iter().find(|g| g.id == object).unwrap();
        assert_eq!((g.creature_id, g.army.unwrap()[0].count), (Some(id), count));
    }
    for edit in [
        NeutralEdit {
            slot: 0,
            creature_id: None,
            count: 0,
        },
        NeutralEdit {
            slot: 1,
            creature_id: Some(60),
            count: 1,
        },
        NeutralEdit {
            slot: 0,
            creature_id: Some(99999),
            count: 1,
        },
    ] {
        assert!(before.plan_neutral_edits(ghost.id, &[edit]).is_err());
    }
    let original = open("SaAndVla.GM2").unwrap().neutral_armies();
    println!(
        "original verified wandering quantities {}",
        original
            .groups
            .iter()
            .filter(|g| g.class == 54 && g.read_only_reason.is_none())
            .count()
    );
    assert_eq!(
        original
            .groups
            .iter()
            .filter(|g| g.class == 54 && g.read_only_reason.is_none())
            .count(),
        291
    );
}
