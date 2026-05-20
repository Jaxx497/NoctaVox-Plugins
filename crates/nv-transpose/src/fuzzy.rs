use crate::domain::Song;
use std::{cmp::Ordering, time::Duration};
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

pub struct FuzzyMatch {
    pub imported_idx: usize,
    pub candidate_id: u64,
    pub confidence: f32,
}

pub fn resolve_by_headers(
    library: &[Song],
    candidates: &[(usize, &Song)],
) -> (Vec<(usize, u64)>, Vec<usize>) {
    let matches = try_fuzzy_match(&library, candidates);
    let mut valid = Vec::new();
    let mut invalid = Vec::new();

    for m in matches {
        match m.confidence >= 0.75 {
            true => valid.push((m.imported_idx, m.candidate_id)),
            false => invalid.push(m.imported_idx),
        }
    }

    (valid, invalid)
}

fn normalize(s: &str) -> String {
    s.nfd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
}

pub struct NormalizedSong {
    pub id: Option<u64>,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: Duration,
}

impl NormalizedSong {
    fn from_song(s: &Song) -> Self {
        Self {
            id: s.id,
            title: normalize(&s.title),
            artist: normalize(&s.artist),
            album: normalize(&s.album),
            duration: s.duration,
        }
    }
}

pub fn try_fuzzy_match(library: &[Song], misses: &[(usize, &Song)]) -> Vec<FuzzyMatch> {
    if library.is_empty() || misses.is_empty() {
        return Vec::new();
    }

    let lib_n: Vec<NormalizedSong> = library.iter().map(NormalizedSong::from_song).collect();

    let imp_n: Vec<(usize, NormalizedSong)> = misses
        .iter()
        .map(|(idx, s)| (*idx, NormalizedSong::from_song(s)))
        .collect();

    let mut matches = Vec::with_capacity(imp_n.len());

    for (idx, song) in &imp_n {
        let best = lib_n
            .iter()
            .map(|s| (s, score(song, s)))
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Ordering::Equal));

        if let Some((candidate, confidence)) = best {
            if let Some(id) = candidate.id {
                matches.push(FuzzyMatch {
                    imported_idx: *idx,
                    candidate_id: id,
                    confidence,
                });
            }
        }
    }

    matches
}

fn score(imported: &NormalizedSong, candidate: &NormalizedSong) -> f32 {
    let mut score = 0.0;

    if &imported.title == &candidate.title {
        score += 0.50
    }

    if !imported.artist.is_empty() && &imported.artist == &candidate.artist {
        score += 0.15
    }

    if !imported.album.is_empty() && &imported.album == &candidate.album {
        score += 0.15
    }

    match duration_ratio(imported.duration, candidate.duration) {
        r if r >= 0.99 => score += 0.20,
        r if r >= 0.95 => score += 0.10,
        _ => {}
    }

    score
}

fn duration_ratio(a: Duration, b: Duration) -> f32 {
    let (a, b) = (a.as_secs_f32(), b.as_secs_f32());
    if a == 0.0 || b == 0.0 {
        return 0.0;
    }

    if a < b { a / b } else { b / a }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib_song(id: u64, title: &str, artist: &str, album: &str, duration_secs: u64) -> Song {
        Song {
            id: Some(id),
            title: title.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            duration: Duration::from_secs(duration_secs),
            path: String::new(),
        }
    }

    fn imp_song(title: &str, artist: &str, album: &str, duration_secs: u64) -> Song {
        Song {
            id: None,
            title: title.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            duration: Duration::from_secs(duration_secs),
            path: String::new(),
        }
    }

    #[test]
    fn normalize_lowercases() {
        assert_eq!(normalize("Hello WORLD"), "hello world");
    }

    #[test]
    fn normalize_strips_diacritics() {
        assert_eq!(normalize("Beyoncé"), normalize("Beyonce"));
        assert_eq!(normalize("Björk"), normalize("bjork"));
    }

    #[test]
    fn normalize_combines_case_and_diacritics() {
        assert_eq!(normalize("BEYONCÉ"), normalize("beyonce"));
    }

    #[test]
    fn duration_ratio_is_symmetric() {
        let a = duration_ratio(Duration::from_secs(180), Duration::from_secs(200));
        let b = duration_ratio(Duration::from_secs(200), Duration::from_secs(180));
        assert!((a - b).abs() < 1e-6);
    }

    #[test]
    fn try_fuzzy_match_picks_best_candidate() {
        let library = vec![
            lib_song(1, "Other Song", "", "", 200),
            lib_song(2, "Karma Police", "Radiohead", "OK Computer", 261),
            lib_song(3, "Different Track", "", "", 100),
        ];
        let imported = imp_song("Karma Police", "Radiohead", "OK Computer", 261);
        let misses = vec![(7, &imported)];

        let matches = try_fuzzy_match(&library, &misses);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].imported_idx, 7);
        assert_eq!(matches[0].candidate_id, 2);
        assert!((matches[0].confidence - 1.0).abs() < 1e-6);
    }

    #[test]
    fn try_fuzzy_match_handles_empty_misses() {
        let library = vec![lib_song(1, "Track", "", "", 200)];
        let matches = try_fuzzy_match(&library, &[]);
        assert!(matches.is_empty());
    }

    #[test]
    fn try_fuzzy_match_handles_empty_library() {
        let imported = imp_song("Track", "", "", 200);
        let matches = try_fuzzy_match(&[], &[(0, &imported)]);
        assert!(matches.is_empty());
    }

    #[test]
    fn try_fuzzy_match_preserves_imported_idx() {
        let library = vec![lib_song(1, "Track", "", "", 200)];
        let imported = imp_song("Track", "", "", 200);
        let misses = vec![(42, &imported)];

        let matches = try_fuzzy_match(&library, &misses);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].imported_idx, 42);
    }

    #[test]
    fn resolve_by_headers_splits_above_and_below_threshold() {
        let library = vec![
            lib_song(1, "Strong Match", "Right Artist", "Right Album", 200),
            lib_song(2, "Weak Match", "Wrong Artist", "Wrong Album", 999),
        ];
        let strong = imp_song("Strong Match", "Right Artist", "Right Album", 200);
        let weak = imp_song("Weak Match", "", "", 0);

        let misses = vec![(0, &strong), (1, &weak)];
        let (resolved, unresolved) = resolve_by_headers(&library, &misses);

        // Strong match scores 1.0 (>=0.75)
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0], (0, 1));

        // Weak match best-candidate is "Weak Match" with title-only (0.50, <0.75)
        assert_eq!(unresolved, vec![1]);
    }

    #[test]
    fn resolve_by_headers_handles_no_candidates() {
        let library = vec![];
        let imported = imp_song("Whatever", "", "", 100);
        let (resolved, unresolved) = resolve_by_headers(&library, &[(3, &imported)]);

        assert!(resolved.is_empty());
        // With an empty library, try_fuzzy_match returns no FuzzyMatch entries,
        // so the "still missing" branch never fires either. Document this behavior:
        assert!(unresolved.is_empty());
    }
}
