//! Native defeat evidence must not be mistaken for surviving quantity evidence.
use h3forge_save::{SaveDocument, neutral::NeutralEdit};

#[test]
fn native_ghost_defeat_removes_one_current_group_without_proving_quantity() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping ghost defeat pair: H3FORGE_FIXTURE_ROOT unset");
        return;
    };
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.local/fixtures/neutral-defeat-test");
    std::fs::create_dir_all(&dir).unwrap();
    let open = |phase: &str| {
        let name = format!("SaAndVla_{phase}_defeat.GM2");
        let source = std::path::PathBuf::from(&root).join(&name);
        std::fs::copy(source, dir.join(&name)).unwrap();
        SaveDocument::open(std::fs::read(dir.join(name)).unwrap()).unwrap()
    };
    let before = open("before");
    let after = open("after");
    assert_eq!((before.format.major, before.format.minor), (44, 11));
    assert_eq!(before.map.name_raw, after.map.name_raw);
    let b = before.neutral_armies();
    let a = after.neutral_armies();
    assert_eq!(b.unavailable_reason, None);
    assert_eq!(a.unavailable_reason, None);
    let monsters = |items: &h3forge_save::neutral::NeutralArmies| {
        items.groups.iter().filter(|g| g.class == 54).count()
    };
    assert_eq!((monsters(&b), monsters(&a)), (1186, 1185));
    let lost: Vec<_> = b
        .groups
        .iter()
        .filter(|g| g.class == 54 && !a.groups.iter().any(|other| other.id == g.id))
        .collect();
    assert_eq!(lost.len(), 1);
    let lost = lost[0];
    assert_eq!(
        (lost.id, lost.creature_id, lost.position),
        (13926, Some(60), [62, 53, 0])
    );
    // Exact tile content: type 60, current object index 13926, property word 517.
    assert_eq!(
        &before.payload()[392279..392289],
        &[54, 0, 60, 0, 102, 54, 5, 2, 0, 0]
    );
    // The template, object and property region is byte-identical after the
    // 24-byte loss of render entries, up to the first changed hero byte.
    assert_eq!(
        &before.payload()[3551767..4394911],
        &after.payload()[3551743..4394887]
    );
    for survivor in b
        .groups
        .iter()
        .filter(|g| g.class == 54 && g.id != lost.id)
        .take(2)
    {
        let other = a.groups.iter().find(|g| g.id == survivor.id).unwrap();
        assert_eq!(
            (survivor.creature_id, survivor.position),
            (other.creature_id, other.position)
        );
        assert!(survivor.army.is_some());
        assert!(
            before
                .plan_neutral_edits(
                    survivor.id,
                    &[NeutralEdit {
                        slot: 0,
                        creature_id: None,
                        count: 0
                    }]
                )
                .is_err()
        );
    }
    assert!(
        after
            .plan_neutral_edits(
                lost.id,
                &[NeutralEdit {
                    slot: 0,
                    creature_id: Some(60),
                    count: 1
                }]
            )
            .is_err()
    );
    println!(
        "Native pair: 1186 -> 1185 wandering groups; removed object 13926, type 60 at 62/53/0. Property region unchanged after render alignment."
    );
}
