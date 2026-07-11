use anyhow::{anyhow, Result};
use csv::Reader;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Minimum match score (0.0–1.0) required to accept a fuzzy title lookup.
const MATCH_THRESHOLD: f64 = 0.6;

/// Master list used by titles mode when `--master` isn't given.
const DEFAULT_MASTER: &str = "Stones Setlist - Master List.csv";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Song {
    song: String,
    key: String,
    tuning: Tuning,
    capo: Option<u8>,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Serialize)]
enum Tuning {
    E,
    G,
    Std,
}

impl<'de> Deserialize<'de> for Tuning {
    // The tuning column sometimes carries a second, comma-separated value
    // (e.g. "STD,E"). We only care about the primary tuning, so keep the first
    // value and ignore anything after the comma. Trimming also tolerates the
    // stray trailing space some rows have (e.g. "G ").
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        let primary = raw.split(',').next().unwrap_or("").trim();
        match primary {
            "E" => Ok(Tuning::E),
            "G" => Ok(Tuning::G),
            "STD" => Ok(Tuning::Std),
            other => Err(serde::de::Error::custom(format!(
                "unknown tuning: {:?}",
                other
            ))),
        }
    }
}

/// Load a setlist/master CSV. The files carry a throwaway "super-header" line
/// above the real `Song,Key,Tuning,Capo,...` header, so we skip one line first.
fn load_songs(path: &str) -> Result<Vec<Song>> {
    let mut f = BufReader::new(File::open(path)?);
    {
        let mut dummy = String::new();
        f.read_line(&mut dummy)?;
    }

    let mut songs = Vec::new();
    for song in Reader::from_reader(f).into_deserialize::<Song>() {
        songs.push(song?);
    }
    Ok(songs)
}

/// Read a plain-text list of song titles, one per line (blank lines ignored).
fn read_titles(path: &str) -> Result<Vec<String>> {
    let f = BufReader::new(File::open(path)?);
    let mut titles = Vec::new();
    for line in f.lines() {
        let line = line?;
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            titles.push(trimmed.to_string());
        }
    }
    Ok(titles)
}

/// Collapse a title down to lowercase alphanumerics so that punctuation,
/// spacing and capitalization don't have to match. "&" becomes "and" first,
/// e.g. `Doo Doo Doo (Heartbreaker)` -> `doodoodooheartbreaker`.
fn normalize(s: &str) -> String {
    let s = s.replace('&', " and ");
    let mut out = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        }
    }
    out
}

/// Classic Levenshtein edit distance between two char sequences.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];

    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Find the catalog song whose title best matches `query`, returning it along
/// with a 0.0–1.0 confidence score. Exact (normalized) hits score 1.0; a title
/// wholly contained in another is treated as a strong match; otherwise the
/// score comes from edit distance.
fn best_match<'a>(query: &str, catalog: &'a [Song]) -> Option<(&'a Song, f64)> {
    let nq = normalize(query);
    if nq.is_empty() {
        return None;
    }

    let mut best: Option<(&Song, f64)> = None;
    for song in catalog {
        let nt = normalize(&song.song);
        if nt.is_empty() {
            continue;
        }

        let score = if nq == nt {
            1.0
        } else {
            let dist = levenshtein(&nq, &nt);
            let max_len = nq.len().max(nt.len());
            let edit_score = 1.0 - dist as f64 / max_len as f64;
            if nt.contains(&nq) || nq.contains(&nt) {
                edit_score.max(0.9)
            } else {
                edit_score
            }
        };

        if best.map_or(true, |(_, bs)| score > bs) {
            best = Some((song, score));
        }
    }
    best
}

/// Print songs in order, inserting a TUNING BREAK whenever the capo position
/// changes within a given tuning.
fn print_setlist(songs: &[Song]) {
    let mut guitars: HashMap<Tuning, Option<u8>> = HashMap::new();
    for song in songs {
        let prev_capo = guitars.entry(song.tuning).or_insert(song.capo);
        if song.capo != *prev_capo {
            println!("\nTUNING BREAK\n");
            *prev_capo = song.capo;
        }
        println!("{} -{}-", song.song, song.key);
    }
}

const USAGE: &str = "usage:
  setlist <setlist.csv>
  setlist --titles <titles.txt> [--master <master.csv>]";

fn main() -> Result<()> {
    let argv: Vec<String> = std::env::args().skip(1).collect();

    let mut titles_path: Option<String> = None;
    let mut master_path: Option<String> = None;
    let mut positional: Option<String> = None;

    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "-t" | "--titles" => {
                i += 1;
                titles_path = argv.get(i).cloned();
            }
            "-m" | "--master" => {
                i += 1;
                master_path = argv.get(i).cloned();
            }
            other => positional = Some(other.to_string()),
        }
        i += 1;
    }

    match (titles_path, master_path, positional) {
        // Titles mode: look each requested title up in the master catalog.
        // `--master` is optional and defaults to DEFAULT_MASTER.
        (Some(titles_path), master_path, _) => {
            let master_path = master_path.unwrap_or_else(|| DEFAULT_MASTER.to_string());
            let catalog = load_songs(&master_path)?;
            let titles = read_titles(&titles_path)?;

            let mut chosen = Vec::new();
            for title in &titles {
                match best_match(title, &catalog) {
                    Some((song, score)) if score >= MATCH_THRESHOLD => {
                        if normalize(title) != normalize(&song.song) {
                            eprintln!(
                                "matched \"{}\" -> \"{}\" ({:.0}%)",
                                title,
                                song.song,
                                score * 100.0
                            );
                        }
                        chosen.push(song.clone());
                    }
                    Some((song, score)) => eprintln!(
                        "WARNING: no confident match for \"{}\" (closest: \"{}\", {:.0}%) — skipped",
                        title,
                        song.song,
                        score * 100.0
                    ),
                    None => eprintln!("WARNING: no match for \"{}\" — skipped", title),
                }
            }

            print_setlist(&chosen);
        }

        // `--master` on its own has nothing to look up.
        (None, Some(_), _) => {
            return Err(anyhow!("--master requires --titles\n\n{}", USAGE));
        }

        // Original mode: print a ready-made setlist CSV as-is.
        (None, None, Some(path)) => {
            let songs = load_songs(&path)?;
            print_setlist(&songs);
        }

        (None, None, None) => return Err(anyhow!("{}", USAGE)),
    }

    Ok(())
}
