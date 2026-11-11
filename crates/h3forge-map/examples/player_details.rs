//! Focused private-map inspection for H3F-023; no output files or writes.
fn main() {
    let path = std::env::args().nth(1).expect("map path");
    let map = h3forge_map::read_map(std::path::Path::new(&path), Default::default()).unwrap();
    println!(
        "dimensions {} underground {}",
        map.header.size, map.header.has_underground
    );
    for (color, p) in map.header.players.iter().enumerate() {
        println!(
            "{color}: human={} computer={} team={:?} main-town={:?} random-hero={} main-hero={:?}",
            p.can_human_play,
            p.can_computer_play,
            p.team,
            p.main_town,
            p.has_random_hero,
            p.main_hero
        );
        if let Ok(body) = &map.body {
            for t in body
                .towns
                .iter()
                .filter(|t| Some(t.entrance()) == p.main_town || Some(t.anchor) == p.main_town)
            {
                println!(
                    "  town owner={:?} faction={:?} anchor={:?} entrance={:?}",
                    t.owner,
                    t.faction,
                    t.anchor,
                    t.entrance()
                );
            }
        }
    }
    let body = map.body.unwrap();
    println!(
        "obelisks {:?}; grails {:?}; trailing {}",
        body.obelisks, body.grails, body.trailing_bytes
    );
}
