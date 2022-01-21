#[allow(dead_code)]
mod game;
mod word_list;

use std::{
    collections::{hash_map::RandomState, HashMap, HashSet},
    io::Write,
    iter::FromIterator,
};

use devtimer::DevTime;
use game::{CheckData, LetterResult};
use prettytable::{cell, row, Table};
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};
use word_list::WordList;

use crate::game::Game;

#[derive(Clone, Debug)]
struct DictionarySet {
    position_maps: [HashMap<char, Vec<&'static str>>; 5],
    contains_map: HashMap<char, HashSet<&'static str>>,
}

impl DictionarySet {
    pub fn new() -> Self {
        let mut prototype = HashMap::<char, Vec<&'static str>>::new();
        let mut contains_map = HashMap::<char, HashSet<&'static str>>::new();
        for c in 'a'..='z' {
            prototype.insert(c, Default::default());
            contains_map.insert(c, Default::default());
        }

        DictionarySet {
            position_maps: [
                prototype.clone(),
                prototype.clone(),
                prototype.clone(),
                prototype.clone(),
                prototype.clone(),
            ],
            contains_map,
        }
    }
}

async fn build_dictionaries(word_list: &WordList) -> DictionarySet {
    let mut timer = DevTime::new_simple();
    timer.start();
    let initial = DictionarySet::new();
    let list = word_list.get();
    list.iter().fold(initial, |mut set, item| {
        for i in 0..5 {
            let c = &item.chars().nth(i).unwrap();
            set.position_maps[i].get_mut(c).unwrap().push(item);
            set.contains_map.get_mut(c).unwrap().insert(item);
        }
        set
    })
}

fn calculate_score(dictionary: &DictionarySet, word: &'static str) -> usize {
    const CONTAINS_VALUE: usize = 0;
    const POSITION_VALUE: usize = 1;
    let contains_count: usize = HashSet::<char, RandomState>::from_iter(word.chars())
        .iter()
        .map(|c| dictionary.contains_map.get(&c).map_or(0, HashSet::len))
        .sum();
    let position_count: usize = word
        .chars()
        .enumerate()
        .map(|(i, c)| dictionary.position_maps[i].get(&c).map_or(0, Vec::len))
        .sum();
    (contains_count * CONTAINS_VALUE) + (position_count * POSITION_VALUE)
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), std::io::Error> {
    let word_list = WordList::new();
    // let word_list = word_list
    //     .remove_letter('l')
    //     .remove_letter('a')
    //     .ensure_letter('r')
    //     .remove_letter('e')
    //     .remove_letter('s');
    // let word_list = word_list
    //     .remove_letter('f')
    //     .ensure_letter('r')
    //     .remove_letter('u')
    //     .remove_letter('t')
    //     .remove_letter('y')
    //     .ensure_letter('i');
    // let word_list = word_list
    //     .remove_letter('o')
    //     .ensure_letter('i')
    //     .remove_letter('n')
    //     .remove_letter('y');
    // let word_list = word_list.remove_letter('g').remove_letter('a');
    // let word_list = word_list.remove_letter('u').remove_letter('d');
    // let word_list = word_list.remove_letter('g');

    let set = build_dictionaries(&word_list).await;
    // let word_list = word_list.intersect(set.position_maps[0].get(&'p').unwrap().to_vec());
    // let word_list = word_list.intersect(set.position_maps[1].get(&'r').unwrap().to_vec());
    // let word_list = word_list.intersect(set.position_maps[2].get(&'i').unwrap().to_vec());
    // let word_list = word_list.intersect(set.position_maps[3].get(&'c').unwrap().to_vec());
    // let word_list = word_list.intersect(set.position_maps[4].get(&'t').unwrap().to_vec());

    // let word_list = word_list.subtract(set.position_maps[3].get(&'i').unwrap().to_vec());
    // let word_list = word_list.subtract(set.position_maps[2].get(&'r').unwrap().to_vec());

    let mut timer = DevTime::new_simple();
    timer.start();
    let set = build_dictionaries(&word_list).await;
    timer.stop();
    println!("build_dictionaries: {} ms", timer.time_in_millis().unwrap());
    // let histo: Vec<_> = set
    //     .position_maps
    //     .iter()
    //     .enumerate()
    //     .map(|(i, m)| {
    //         (
    //             i,
    //             m.iter()
    //                 .map(|(letter, list)| (letter, list.len()))
    //                 .collect::<HashMap<_, _>>(),
    //         )
    //     })
    //     .collect();

    let mut histo: Vec<_> = set.contains_map.iter().map(|(c, m)| (c, m.len())).collect();
    histo.sort_by_key(|item| item.1);
    histo.reverse();
    println!("Word count: {}", word_list.word_count());
    // println!("{:?}", histo);

    let mut word_cache: HashMap<Vec<char>, usize> = Default::default();
    let words = word_list.get();
    let ideal = word_list.word_count() >> 5;
    println!("Ideal: {}", ideal);
    let mut reduction = words
        .iter()
        .map(|word| {
            let mut sorted_chars = Vec::from_iter(word.chars());
            sorted_chars.sort();

            let remaining_words = match word_cache.get(&sorted_chars) {
                Some(remaining) => *remaining,
                None => {
                    let remaining = sorted_chars
                        .iter()
                        .fold(word_list.clone(), |list, c| list.whittle(*c))
                        .word_count();
                    word_cache.insert(sorted_chars, remaining);
                    remaining
                }
            };
            (*word, remaining_words, calculate_score(&set, word))
        })
        .collect::<Vec<_>>();
    reduction.sort_by_key(|x| (x.1, (1 << 32) - x.2, x.0));
    reduction.truncate(10);
    print_reduction(&reduction)?;

    // let words = word_list.get();
    // let mut scores = words
    //     .iter()
    //     .map(|word| (word, calculate_score(&set, word)))
    //     .collect::<Vec<_>>();
    // scores.sort_by_key(|t| t.1);
    // scores.reverse();
    // scores.truncate(20);
    // println!("{:?}", scores);

    let mut game = Game::new("robot".to_string());
    print_results_no_spoiler(&game.check("lares"))?;
    Ok(())
}

fn print_reduction(reduction: &Vec<(&str, usize, usize)>) -> Result<(), std::io::Error> {
    let mut table = Table::new();
    table.add_row(row!["Word", "Remaining", "Pos Score"]);
    for (word, remaining, score) in reduction {
        table.add_row(row![word, remaining, score]);
    }
    table.printstd();
    Ok(())
}

fn print_results(result: &CheckData) -> Result<(), std::io::Error> {
    let mut stdout = StandardStream::stdout(termcolor::ColorChoice::Auto);
    for letter in result.letters.iter() {
        match letter {
            game::LetterResult::Exact(c) => {
                stdout.set_color(
                    ColorSpec::new()
                        .set_intense(true)
                        .set_fg(Some(Color::Green)),
                )?;
                write!(&mut stdout, "{}", c.to_ascii_uppercase())?;
                stdout.reset()?;
            }
            game::LetterResult::Contains(c) => {
                stdout.set_color(
                    ColorSpec::new()
                        .set_intense(true)
                        .set_fg(Some(Color::Yellow)),
                )?;
                write!(&mut stdout, "{}", c.to_ascii_uppercase())?;
                stdout.reset()?;
            }
            game::LetterResult::NotFound(c) => {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)))?;
                write!(&mut stdout, "{}", c.to_ascii_uppercase())?;
                stdout.reset()?;
            }
        }
    }
    writeln!(&mut stdout, ": {}/6", result.guesses)?;
    Ok(())
}

fn print_results_no_spoiler(result: &CheckData) -> Result<(), std::io::Error> {
    let mut stdout = StandardStream::stdout(termcolor::ColorChoice::Auto);
    println!("{:?}", result);
    writeln!(&mut stdout, "Wordle {}/6", result.guesses)?;

    for letter in result.letters.iter() {
        match letter {
            LetterResult::Exact(_) => write!(&mut stdout, "🟩")?,
            LetterResult::Contains(_) => write!(&mut stdout, "🟨")?,
            LetterResult::NotFound(_) => write!(&mut stdout, "⬛")?,
        };
        stdout.reset()?;
    }
    writeln!(&mut stdout, "")?;
    Ok(())
}

// Wordle 215 3/6

// ⬛⬛🟨⬛⬛
// 🟨🟨⬛⬛🟩
// 🟩🟩🟩🟩🟩
