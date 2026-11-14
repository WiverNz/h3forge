//! H3F-030 manual experiment, deliberately restricted to fingerprinted samples.
//! This changes initialized teams only. Runtime alliance behavior is unverified.
use h3forge_save::{BytePatch, SaveDocument, apply_patches, compress_gzip};
use sha2::{Digest, Sha256};
use std::{error::Error, fs, io::Write, path::Path};

fn sample_layout(bytes: &[u8]) -> Result<(usize, u8), Box<dyn Error>> {
    let hash = format!("{:x}", Sha256::digest(bytes));
    match hash.as_str() {
        "8d232da61946a960b0813defd67c19979ac7d3604aafd4bb5bb3d6b1d0caf25c"
        | "852640d9ffe55960b0356ef3d31b344555de294e4e2e91373eb0bfc78fe2134a"
        | "ab5e3c3142337e1d10e38975bae3f195d5475a7ed01ab5bd9083cafa9a1aa493" => Ok((474, 0)),
        "9594a96725e5f692274aced96efd0e6db42d6e30cf43ee81587d3b40ff3342ad" => Ok((476, 1)),
        _ => Err(
            "Only the four original H3F-030 samples are supported; unknown or edited input refused"
                .into(),
        ),
    }
}

fn experiment(bytes: Vec<u8>, color: usize, team: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    if ![0, 1, 2, 4].contains(&color) || !(1..=2).contains(&team) {
        return Err(
            "Use a playable color (0 Red, 1 Blue, 2 Tan, 4 Orange) and existing team 1 or 2".into(),
        );
    }
    let (count_at, blue) = sample_layout(&bytes)?;
    let doc = SaveDocument::open(bytes)?;
    let expected = [2, 0, blue, 1, 0, 1, 0, 0, 0];
    if doc.payload().get(count_at..count_at + 9) != Some(expected.as_slice()) {
        return Err("Initialized team header does not match sample evidence".into());
    }
    let offset = count_at + 1 + color;
    let old = doc.payload()[offset];
    let new = team - 1;
    if old == new {
        return Err("Player is already initialized in that team".into());
    }
    let mut payload = doc.payload().to_vec();
    apply_patches(
        &mut payload,
        &[BytePatch::new(
            offset,
            vec![old],
            vec![new],
            "EXPERIMENTAL initialized team reassignment",
        )?],
    )?;
    // No count/renumbering policy: both existing teams must retain participants.
    for id in 0..2 {
        if ![0, 1, 2, 4].iter().any(|c| payload[count_at + 1 + c] == id) {
            return Err("Experiment cannot empty a team".into());
        }
    }
    let encoded = compress_gzip(&payload);
    let reopened = SaveDocument::open_strict(encoded.clone())?;
    if reopened.payload() != payload
        || payload
            .iter()
            .zip(doc.payload())
            .filter(|(a, b)| a != b)
            .count()
            != 1
    {
        return Err("Exact one-byte round-trip verification failed".into());
    }
    Ok(encoded)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    // Never truncate or replace a destination, including a source or hard link.
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    let persisted = fs::read(path)?;
    if persisted != bytes {
        return Err("Output readback differs".into());
    }
    SaveDocument::open_strict(persisted)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 5 {
        return Err(
            "Usage: experimental_teams INPUT.GM2 NEW_OUTPUT.GM2 COLOR_INDEX TEAM_NUMBER (1 or 2)"
                .into(),
        );
    }
    let color = args[3].to_str().ok_or("Invalid color")?.parse()?;
    let team = args[4].to_str().ok_or("Invalid team")?.parse()?;
    let output = experiment(fs::read(&args[1])?, color, team)?;
    write_new(Path::new(&args[2]), &output)?;
    println!(
        "Created {}. Exactly one initialized-team byte changed; HotA runtime behavior is UNVERIFIED.",
        Path::new(&args[2]).display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_input_and_invalid_requests() {
        assert!(experiment(vec![0; 1024], 1, 2).is_err());
        assert!(experiment(vec![], 3, 2).is_err());
        assert!(experiment(vec![], 1, 3).is_err());
    }

    #[test]
    fn never_overwrites_existing_file() {
        let path = std::env::temp_dir().join(format!("h3forge-teams-{}.tmp", std::process::id()));
        let mut original = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        original.write_all(b"original").unwrap();
        drop(original);
        assert!(write_new(&path, b"replacement").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
        fs::remove_file(path).unwrap();
    }
}
