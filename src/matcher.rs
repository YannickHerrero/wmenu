use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use crate::index::AppEntry;
use crate::mru::Mru;

const MRU_WEIGHT: f32 = 100.0;

pub struct Engine {
    matcher: Matcher,
    buf: Vec<char>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT),
            buf: Vec::new(),
        }
    }

    pub fn search(&mut self, query: &str, entries: &[AppEntry], mru: &Mru) -> Vec<usize> {
        if query.trim().is_empty() {
            let mut idxs: Vec<usize> = (0..entries.len()).collect();
            idxs.sort_by(|&a, &b| {
                mru.boost(&entries[b].path)
                    .total_cmp(&mru.boost(&entries[a].path))
            });
            return idxs;
        }
        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
        let mut scored: Vec<(usize, u32)> = entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let haystack = Utf32Str::new(&e.name, &mut self.buf);
                let score = pattern.score(haystack, &mut self.matcher)?;
                let boost = (mru.boost(&e.path) * MRU_WEIGHT) as u32;
                Some((i, score + boost))
            })
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().map(|(i, _)| i).collect()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
