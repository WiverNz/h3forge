//! Opt-in H3F-030 team regression on private saves. Reads originals only;
//! nothing is written anywhere.
use h3forge_save::{SaveDocument, TeamEdit, apply_patches, compress_gzip};
use std::path::{Path, PathBuf};

fn open(path: PathBuf) -> SaveDocument {
    SaveDocument::open(std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn sample_teams_structural_table_and_checked_moves() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping private team fixtures: set H3FORGE_FIXTURE_ROOT");
        return;
    };
    let root = PathBuf::from(root);
    // (save, assignment offset, count, teams, participating colors)
    let cases: [(&str, usize, u8, [u8; 8], &[usize]); 6] = [
        (
            "same_teams",
            475,
            2,
            [0, 0, 1, 0, 1, 0, 0, 0],
            &[0, 1, 2, 4],
        ),
        (
            "same_teams_restart",
            475,
            2,
            [0, 0, 1, 0, 1, 0, 0, 0],
            &[0, 1, 2, 4],
        ),
        (
            "separate_teams",
            475,
            2,
            [0, 0, 1, 0, 1, 0, 0, 0],
            &[0, 1, 2, 4],
        ),
        (
            "separate_team_map_2",
            477,
            2,
            [0, 1, 1, 0, 1, 0, 0, 0],
            &[0, 1, 2, 4],
        ),
        // Special victory condition and three custom heroes before the anchors.
        (
            "SaAndVla",
            431,
            2,
            [0, 1, 1, 1, 1, 1, 1, 0],
            &[0, 1, 2, 3, 4, 5, 6, 7],
        ),
        // Named starting heroes, five teams, 214 custom heroes.
        (
            "AUTOSAVE",
            528,
            5,
            [0, 1, 2, 2, 3, 1, 3, 4],
            &[0, 1, 2, 3, 4, 5, 6, 7],
        ),
    ];
    for (name, offset, count, teams, colors) in cases {
        let doc = open(root.join(format!("{name}.GM2")));
        let table = doc.team_table().unwrap_or_else(|| panic!("{name}"));
        assert_eq!(table.assignments_offset(), Some(offset), "{name}");
        assert_eq!((table.count, table.teams), (count, teams), "{name}");
        let participating: Vec<usize> = (0..8).filter(|&c| table.participating[c]).collect();
        assert_eq!(participating, colors, "{name}");
        assert_eq!(doc.payload()[offset - 1], count);

        // Every legal single move is exactly one checked byte at offset + color.
        for &color in colors {
            for team in 0..count {
                let edit = TeamEdit {
                    color: color as u8,
                    team,
                };
                let alone = colors.iter().filter(|&&c| teams[c] == teams[color]).count() == 1;
                let planned = doc.plan_team_edits(&[edit]);
                if team == teams[color] {
                    assert!(planned.unwrap().is_empty());
                } else if alone {
                    assert!(planned.is_err(), "{name}: emptied a team");
                } else {
                    let patches = planned.unwrap();
                    assert_eq!(patches.len(), 1);
                    assert_eq!(patches[0].offset, offset + color);
                    let mut payload = doc.payload().to_vec();
                    apply_patches(&mut payload, &patches).unwrap();
                    let differing: Vec<usize> = (0..payload.len())
                        .filter(|&i| payload[i] != doc.payload()[i])
                        .collect();
                    assert_eq!(differing, [offset + color]);
                    let reopened = SaveDocument::open_strict(compress_gzip(&payload)).unwrap();
                    assert_eq!(reopened.payload(), payload);
                    assert_eq!(reopened.team_table().unwrap().teams[color], team);
                    assert_eq!(reopened.players.len(), doc.players.len());
                }
            }
        }
        // Gates: new team, absent color, duplicate operation.
        assert!(
            doc.plan_team_edits(&[TeamEdit {
                color: colors[0] as u8,
                team: count
            }])
            .is_err()
        );
        if let Some(absent) = (0..8u8).find(|&c| !table.participating[c as usize]) {
            assert!(
                doc.plan_team_edits(&[TeamEdit {
                    color: absent,
                    team: 0
                }])
                .is_err()
            );
        }
        assert!(
            doc.plan_team_edits(&[TeamEdit { color: 8, team: 0 }])
                .is_err()
        );
    }

    // Swapping two single-player teams is legal only as one joint plan.
    let doc = open(root.join("AUTOSAVE.GM2"));
    let swap = [
        TeamEdit { color: 0, team: 4 },
        TeamEdit { color: 7, team: 0 },
    ];
    assert!(doc.plan_team_edits(&swap[..1]).is_err());
    assert_eq!(doc.plan_team_edits(&swap).unwrap().len(), 2);
    assert!(doc.plan_team_edits(&[swap[0], swap[0]]).is_err());

    // The planner reproduces the two saves that were loaded, played for a
    // turn, saved and reloaded in HotA; the game kept the edited assignment.
    let verified =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.local/fixtures/teams-030-experimental");
    for (source, edited, after, team) in [
        (
            "same_teams_restart",
            "Blue_join_Tan_Orange",
            "Blue_join_Tan_Orange_after",
            1,
        ),
        (
            "separate_team_map_2",
            "Blue_join_Red",
            "Blue_join_Red_after",
            0,
        ),
    ] {
        if !verified.join(format!("{after}.GM2")).exists() {
            eprintln!("skipping game-verified pair {edited}: not present");
            continue;
        }
        let doc = open(root.join(format!("{source}.GM2")));
        let mut payload = doc.payload().to_vec();
        let patches = doc.plan_team_edits(&[TeamEdit { color: 1, team }]).unwrap();
        apply_patches(&mut payload, &patches).unwrap();
        assert_eq!(
            open(verified.join(format!("{edited}.GM2"))).payload(),
            payload
        );
        let native = open(verified.join(format!("{after}.GM2")));
        let mut expected = doc.team_table().unwrap();
        expected.teams[1] = team;
        let kept = native.team_table().unwrap();
        assert_eq!((kept.count, kept.teams), (expected.count, expected.teams));
        assert_eq!(kept.participating, expected.participating);
    }
}
