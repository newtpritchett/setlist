# setlist

A tiny command-line tool that turns a setlist spreadsheet (CSV) into a printable
list of songs, automatically inserting a **TUNING BREAK** wherever the guitar
needs to be re-capoed for a given tuning.

It's meant as a stage cheat-sheet: run the setlist through it and you get, in
playing order, each song with its key — plus a clear marker any time you have to
stop and change your capo position.

## How it works

For every tuning (E, G, STD) the tool remembers the last capo position it saw.
When a song in that same tuning shows up with a *different* capo than the
previous one, it prints a `TUNING BREAK` line before the song. That's your cue
that you can't just roll straight from the last song into this one — the capo has
to move first.

## Building

Requires a [Rust toolchain](https://www.rust-lang.org/tools/install) (Cargo).

```sh
cargo build --release
```

The binary lands at `target/release/setlist`. Cargo does **not** put it on your
`PATH`, so run it by that path — `./target/release/setlist` from the project
directory — rather than as a bare `setlist` command. All the examples below use
that path. (If you'd prefer to type just `setlist`, run `cargo install --path .`
to copy it into `~/.cargo/bin`, then substitute `setlist` for
`./target/release/setlist` everywhere below.)

## Usage

The tool has two modes.

### 1. Print a ready-made setlist CSV

```sh
./target/release/setlist <path-to-setlist.csv>
```

or, without building a release binary:

```sh
cargo run -- <path-to-setlist.csv>
```

Reads a CSV that already contains the songs in playing order (see
[Input format](#input-format)) and prints them.

### 2. Build a setlist from a list of titles

Give it a plain-text file of song titles (one per line), and it looks each title
up in your **master list** and prints them in the order you listed:

```sh
./target/release/setlist --titles <titles.txt>
# short flag:
./target/release/setlist -t <titles.txt>
```

The master list defaults to `Stones Setlist - Master List.csv` in the current
directory. Point it at a different catalog with `--master`/`-m`:

```sh
./target/release/setlist --titles <titles.txt> --master <other-master.csv>
```

Titles **don't have to match exactly** — matching is fuzzy and ignores case,
punctuation, and spacing, and tolerates typos. So `gimmie shelter`,
`cant you hear me knocking`, and `honky tonk woman` all resolve to the right
songs. When a title is matched inexactly, a note is printed to **stderr**:

```
matched "gimmie shelter" -> "Gimme Shelter" (92%)
```

If a title can't be matched confidently it's skipped with a `WARNING` on stderr.
Because those messages go to stderr, the setlist on stdout stays clean and can be
redirected to a file:

```sh
./target/release/setlist -t titles.txt > tonight.txt
```

### Output

Either mode prints the same thing. Each line is `<Song> -<Key>-`, and a blank-padded
`TUNING BREAK` appears whenever the capo changes within a tuning:

```
Start Me Up -C-
Brown Sugar -C-

TUNING BREAK

Wild Horses -G-
```

## Input format

### CSV files (setlist and master list)

Both the setlist CSV and the master-list CSV use the same layout:

- **A throwaway first line.** The very first row of the file is skipped (it's
  treated as a spreadsheet super-header), so the *second* row must be the real
  column header.
- **A header row** naming these columns (PascalCase):

  | Column   | Meaning                                    | Required |
  |----------|--------------------------------------------|----------|
  | `Song`   | Song title                                 | yes      |
  | `Key`    | Musical key                                | yes      |
  | `Tuning` | Guitar tuning — one of `E`, `G`, or `STD`  | yes      |
  | `Capo`   | Capo fret position (a number)              | optional |

  Extra columns after `Capo` (e.g. a notes column) are ignored.

- **One row per song.** For a setlist, order the rows the way you'll play them;
  the master list can be in any order (titles mode reorders it for you).

Notes on values:

- `Tuning` must be `E`, `G`, or `STD`. If the cell has a second, comma-separated
  value (e.g. `STD,E`), only the first is used. A stray trailing space after `G`
  is tolerated. Leave `Capo` blank if the song uses no capo.

A minimal example file (remember the ignored first line):

```csv
My Band's Setlist,,,
Song,Key,Tuning,Capo
Start Me Up,C,E,4
Brown Sugar,C,E,4
Wild Horses,G,G,
```

### Titles file

For titles mode, a plain-text file with one song title per line. Blank lines are
ignored, and titles are matched loosely (case-, punctuation- and
spacing-insensitive, typo-tolerant):

```
brown sugar
gimmie shelter
honky tonk woman
wild horses
```

## Notes / limitations

- The setlist prints to standard output; redirect to a file if you want to save
  or print it: `./target/release/setlist setlist.csv > tonight.txt`. In titles mode, match notes
  and warnings go to standard error, so redirecting stdout still yields a clean
  setlist.
- Rows that don't parse (unknown tuning, non-numeric capo, missing columns) cause
  the program to stop with an error, so keep the columns clean.
- Fuzzy matching picks the single closest master-list title above a confidence
  threshold. A title below the threshold is skipped with a warning; check stderr
  if a song is missing from the output.
