#[allow(dead_code)]
mod game;
mod word_list;

use std::{
    collections::{hash_map::RandomState, BTreeSet, HashMap, HashSet},
    io::Write,
    iter::FromIterator,
};

use clap::Parser;
use devtimer::DevTime;
use game::{CheckData, LetterResult};
use prettytable::{cell, row, Table};
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};
use word_list::WordList;

use crate::game::{Game, GuessResult};

#[derive(Clone, Debug)]
struct DictionarySet {
    position_maps: [HashMap<char, HashSet<&'static str>>; 5],
    contains_map: HashMap<char, HashSet<&'static str>>,
}

impl DictionarySet {
    pub fn new() -> Self {
        let mut prototype = HashMap::<char, HashSet<&'static str>>::new();
        for c in 'a'..='z' {
            prototype.insert(c, Default::default());
        }

        DictionarySet {
            position_maps: [
                prototype.clone(),
                prototype.clone(),
                prototype.clone(),
                prototype.clone(),
                prototype.clone(),
            ],
            contains_map: prototype.clone(),
        }
    }
}

fn build_dictionaries(word_list: &WordList) -> DictionarySet {
    let mut timer = DevTime::new_simple();
    timer.start();
    let initial = DictionarySet::new();
    let list = word_list.get();
    list.iter().fold(initial, |mut set, item| {
        for (i, c) in item.chars().enumerate() {
            set.position_maps[i].get_mut(&c).unwrap().insert(item);
            set.contains_map.get_mut(&c).unwrap().insert(item);
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
        .map(|(i, c)| dictionary.position_maps[i].get(&c).map_or(0, HashSet::len))
        .sum();
    (contains_count * CONTAINS_VALUE) + (position_count * POSITION_VALUE)
}

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    /// Which day's puzzle to try; defaults to today's
    #[clap(long, short)]
    day: Option<usize>,

    /// Use the word for guessing
    #[clap(long, short)]
    word: Option<String>,

    /// Suggest words to try based on previous results
    #[clap(long, short)]
    suggest: bool,

    /// Straight up cheat. You must supply this flag at least three times
    #[clap(long, parse(from_occurrences))]
    cheat: usize,

    ///Use easy mode
    #[clap(long)]
    easy: bool,

    /// Your guesses
    guesses: Vec<String>,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), std::io::Error> {
    let config = Args::parse();
    let day_opt = config.day;
    let mut game = config
        .word
        .map(|w| Game::for_word(&*w))
        .unwrap_or_else(|| Game::new(day_opt));

    if !config.easy {
        game = game.set_hard_mode();
    }

    if config.cheat >= 3 {
        println!("Today's secret word is: {:?}\n", game.word());
    }

    let invalid_guesses: Vec<&String> = config.guesses.iter().filter(|g| g.len() != 5).collect();
    if !invalid_guesses.is_empty() {
        println!("Invalid guesses: {:?}", invalid_guesses);
        return Ok(());
    }

    let (result, word_list) = config.guesses.iter().map(|g| g.to_ascii_lowercase()).fold(
        (GuessResult::Incorrect, WordList::new()),
        |(prev_result, word_list), guess| match prev_result {
            GuessResult::Win | GuessResult::Lose | GuessResult::Invalid(_) => {
                (prev_result, word_list)
            }
            GuessResult::Incorrect => {
                let result = game.check(&guess);
                print_single_guess(&result).unwrap();
                (result.result, process_results(word_list, result.letters))
            }
        },
    );

    println!();

    match result {
        GuessResult::Win => print_results(&game, config.suggest)?,
        GuessResult::Incorrect => {
            if config.suggest {
                print_suggestion(&suggest(build_dictionaries(&word_list), word_list)?)?;
            }
        }
        GuessResult::Lose => print_results(&game, config.suggest)?,
        GuessResult::Invalid(w) => println!("Guess '{}' does not contain all revealed letters.", w),
    }

    Ok(())
}

fn suggest(
    set: DictionarySet,
    word_list: WordList,
) -> Result<Vec<(&'static str, usize, usize)>, std::io::Error> {
    let mut histo: Vec<_> = set.contains_map.iter().map(|(c, m)| (c, m.len())).collect();
    histo.sort_by_key(|item| item.1);
    histo.reverse();
    println!("Words remaining: {}", word_list.word_count());
    let mut word_cache: HashMap<Vec<char>, usize> = Default::default();
    let words = word_list.get();

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
    reduction.truncate(25);
    Ok(reduction)
}

fn position_match(word_list: WordList, set: &DictionarySet, p: usize, c: &char) -> WordList {
    word_list.intersect(get_position_vec(set, p, c))
}

fn position_mismatch(word_list: WordList, set: &DictionarySet, p: usize, c: &char) -> WordList {
    word_list.subtract(get_position_vec(set, p, c))
}

fn get_position_vec(set: &DictionarySet, p: usize, c: &char) -> Vec<&'static str> {
    Vec::from_iter(set.position_maps[p].get(c).unwrap())
        .iter()
        .map(|x| **x)
        .collect()
}

fn print_suggestion(reduction: &Vec<(&str, usize, usize)>) -> Result<(), std::io::Error> {
    let mut table = Table::new();
    table.add_row(row!["Word", "Remaining", "Pos Score"]);
    for (word, remaining, score) in reduction {
        table.add_row(row![word, remaining, score]);
    }
    table.printstd();
    Ok(())
}

fn process_results(word_list: WordList, letters: Vec<LetterResult>) -> WordList {
    let set = build_dictionaries(&word_list);
    let found_letters =
        BTreeSet::from_iter(letters.iter().filter(|r| r.is_found()).map(|r| r.to_char()));

    letters
        .iter()
        .enumerate()
        .fold(word_list, |word_list, (i, result)| match result {
            LetterResult::Exact(c) => position_match(word_list, &set, i, c),
            LetterResult::Contains(c) => position_mismatch(word_list, &set, i, c).ensure_letter(*c),
            LetterResult::NotFound(c) if !found_letters.contains(c) => {
                //TODO: This is not entirely correct, doesn't remove duplicates that aren't found
                word_list.remove_letter(*c)
            }
            _ => word_list,
        })
}

fn print_single_guess(result: &CheckData) -> Result<(), std::io::Error> {
    let mut stdout = StandardStream::stdout(termcolor::ColorChoice::Auto);
    for letter in result.letters.iter() {
        match letter {
            game::LetterResult::Exact(c) => {
                stdout.set_color(
                    ColorSpec::new()
                        .set_intense(true)
                        .set_fg(Some(Color::Black))
                        .set_bg(Some(Color::Green)),
                )?;
                write!(&mut stdout, " {} ", c.to_ascii_uppercase())?;
                stdout.reset()?;
            }
            game::LetterResult::Contains(c) => {
                stdout.set_color(
                    ColorSpec::new()
                        .set_intense(true)
                        .set_fg(Some(Color::Black))
                        .set_bg(Some(Color::Yellow)),
                )?;
                write!(&mut stdout, " {} ", c.to_ascii_uppercase())?;
                stdout.reset()?;
            }
            game::LetterResult::NotFound(c) => {
                stdout.set_color(
                    ColorSpec::new()
                        .set_fg(Some(Color::White))
                        .set_bg(Some(Color::Black)),
                )?;
                write!(&mut stdout, " {} ", c.to_ascii_uppercase())?;
                stdout.reset()?;
            }
        }
    }
    writeln!(&mut stdout)?;
    Ok(())
}

fn print_results(game: &Game, assisted: bool) -> Result<(), std::io::Error> {
    let num_str = game.day().map(|x| x.to_string()).unwrap_or("".to_string());
    let hard_str = if game.is_easy() { "" } else { "*" };
    let assisted_str = if assisted { " TA" } else { "" };
    let guesses = game.guesses();
    println!(
        "Wordle {} {}/6{}{}",
        num_str,
        guesses.len(),
        hard_str,
        assisted_str
    );
    for result in guesses {
        print_single_result_no_spoiler(&result)?;
    }
    Ok(())
}

fn print_single_result_no_spoiler(result: &CheckData) -> Result<(), std::io::Error> {
    let mut stdout = StandardStream::stdout(termcolor::ColorChoice::Auto);

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
