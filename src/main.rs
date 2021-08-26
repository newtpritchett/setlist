use anyhow::Result;
use csv::Reader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Song {
    song: String,
    key: String,
    tuning: Tuning,
    capo: Option<u8>,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
enum Tuning {
    E,
    #[serde(alias = "G ")] // if it's stupid, but it works, it might still be stupid
    G,
    #[serde(rename = "STD")]
    Std,
}

fn main() -> Result<()> {
    
    let argv: Vec<String> = std::env::args().skip(1).collect();

    let targ = &argv[0];
    //let mut f = BufReader::new(File::open("../Stones Setlist - Sheet1.csv")?);
    let mut f = BufReader::new(File::open(targ)?);
    {
        let mut dummy = String::new();
        // skip a weird super-header that the file has for some reason
        f.read_line(&mut dummy)?;
    }

    let mut guitars = HashMap::new();

    let doc = Reader::from_reader(f).into_deserialize::<Song>();
    for song in doc {
        let song = song?;

        let prev_capo = guitars.entry(song.tuning).or_insert(song.capo);
        if song.capo != *prev_capo {
//            println!("\n{:^30}\n", "TUNING BREAK");
            println!("\n{}\n", "TUNING BREAK");
            *prev_capo = song.capo;
        }


        print!("{} -{}-", song.song, song.key);
//        print!("{:^30}\t{}\t{:?}", song.song, song.key, song.tuning);
//        if let Some(capo) = song.capo {
//            print!("\t{}", capo);
//        }
        println!();
    }

    Ok(())
}
