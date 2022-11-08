use std::{convert::TryInto, iter::FromIterator};

use crate::dictionary_set::DictionarySet;
use hash_histogram::HashHistogram;
use itertools::Itertools;
use rayon::{slice::ParallelSliceMut, prelude::*};
use crate::word_list::WordList;

/// The algorithm here is to find out how well words bisect the problem space. For that we create a 5-bit
/// value for each word in the list determining with that letter is contained in the guess. For speed we
/// group words that have the same letters together (i.e. order of the letters is not important) and
/// rate the candidate words by the smallest max bucket (i.e. worst-case performance). Algorithm is O(o^2).
///
/// We also calculate a second value--position score--which indicates how many times a word has an
/// exact-position match. The higher the score the more likely it is a guess will cut the working set
/// dramatically.
pub fn suggest(
    set: DictionarySet,
    word_list: WordList,
    easy: bool,
) -> Result<Vec<(&'static str, usize, i64)>, std::io::Error> {
    println!("Words remaining: {}", word_list.word_count());
    let words = word_list.get();

    let remaining = word_list.word_count();
    if remaining == 1 {
        return Ok(vec![(words.first().unwrap(), 1, 5)]);
    }

    let mut candidates = if easy {
        WordList::new().get()
    } else {
        words.clone()
    };
    candidates.par_sort_by_key(|word| pattern_from_word(*word));

    let grouped = candidates
        .iter()
        .group_by(|word| pattern_from_word(*word))
        .into_iter()
        .map(|(key, group)| (key, group.cloned().collect_vec()))
        .collect_vec();

    let mut reduction = grouped
        .par_iter()
        .flat_map(|(char_pattern, pattern_words)| {
            let mut hist: HashHistogram<u8> = HashHistogram::new();
            for word in words.iter() {
                let bucket = char_pattern.chars().fold(0_u8, |acc, c| {
                    (acc << 1) + if word.contains(c) { 1 } else { 0 }
                });
                hist.bump(&bucket);
            }

            let remaining = hist.iter().map(|(_, count)| *count).max().unwrap_or(0);
            pattern_words
                .into_iter()
                .map(move |w| (*w, remaining))
                .par_bridge()
        })
        .map(|(word, count)| (word, count, calculate_score(&set, word)))
        .collect::<Vec<(&'static str, usize, i64)>>();

    reduction.par_sort_by_key(|(word, count, score)| (*count, -*score, *word));
    Ok(reduction)
}

fn calculate_score(dictionary: &DictionarySet, word: &'static str) -> i64 {
    word.chars()
        .enumerate()
        .map(|(i, c)| {
            dictionary
                .list_for_position(i)
                .get(&c)
                .map_or(0, |v| v.len())
        })
        .sum::<usize>()
        .try_into()
        .unwrap()
}

fn pattern_from_word(word: &str) -> String {
    let mut sorted_chars = Vec::from_iter(word.chars());
    sorted_chars.sort();
    sorted_chars.iter().collect()
}