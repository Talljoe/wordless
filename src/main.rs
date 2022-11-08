mod dictionary_set;
#[allow(dead_code)]
mod game;
mod word_list;
mod suggest;

use std::{collections::BTreeSet, io::Write, iter::FromIterator};

use crate::dictionary_set::DictionarySet;
use crate::game::{Game, GuessResult};
use clap::{ArgGroup, Parser};
use game::{CheckData, LetterResult};
use prettytable::{cell, row, Table};
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};
use word_list::WordList;
use suggest::suggest;

#[derive(Parser, Debug)]
#[clap(version)]
#[clap(group(ArgGroup::new("puzzle").args(&["day", "word"])))]
struct Args {
    /// Which day's puzzle to try; defaults to today's
    #[clap(long, short)]
    day: Option<usize>,

    /// Word to use for the puzzle instead of the default
    #[clap(long, short)]
    word: Option<String>,

    /// Suggest words to try based on previous results
    #[clap(short, long)]
    suggest: bool,

    /// Number of words to suggest (used with "--suggest")
    #[clap(long, value_name = "COUNT", default_value = "20")]
    suggest_count: usize,

    /// Straight up cheat. You must supply this flag at least three times
    #[clap(long, parse(from_occurrences))]
    cheat: usize,

    ///Use easy mode
    #[clap(short, long)]
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
                (result.result, eliminate_words(word_list, result.letters))
            }
        },
    );

    println!();

    match result {
        GuessResult::Win => print_results(&game, config.suggest)?,
        GuessResult::Incorrect => {
            if config.suggest {
                print_suggestion(
                    config.suggest_count,
                    &suggest(
                        DictionarySet::from_word_list(&word_list),
                        word_list,
                        config.easy,
                    )?,
                )?;
            }
        }
        GuessResult::Lose => print_results(&game, config.suggest)?,
        GuessResult::Invalid(w) => println!("Guess '{}' does not contain all revealed letters.", w),
    }

    Ok(())
}

fn eliminate_words(word_list: WordList, letters: Vec<LetterResult>) -> WordList {
    let set = DictionarySet::from_word_list(&word_list);
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

fn position_match(word_list: WordList, set: &DictionarySet, p: usize, c: &char) -> WordList {
    word_list.intersect(get_position_vec(set, p, c))
}

fn position_mismatch(word_list: WordList, set: &DictionarySet, p: usize, c: &char) -> WordList {
    word_list.subtract(get_position_vec(set, p, c))
}

fn get_position_vec(set: &DictionarySet, p: usize, c: &char) -> Vec<&'static str> {
    Vec::from_iter(set.list_for_position(p).get(c).unwrap())
        .iter()
        .map(|x| **x)
        .collect()
}

fn print_suggestion(
    count: usize,
    reduction: &Vec<(&str, usize, i64)>,
) -> Result<(), std::io::Error> {
    let mut table = Table::new();
    table.add_row(row!["Word", "Remaining", "Pos Score"]);
    for (word, remaining, score) in reduction.iter().take(count) {
        table.add_row(row![word, remaining, score]);
    }
    if reduction.len() > count {
        table.add_row(row!["...", "", ""]);
    }

    table.printstd();
    Ok(())
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
        "Wordle {} {}/6{}{}\n",
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
