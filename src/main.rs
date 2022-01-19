mod game;
mod word_list;

use std::{
    collections::{hash_map::RandomState, HashMap, HashSet},
    io::Write,
    iter::FromIterator,
};

use game::CheckData;
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
    let set = build_dictionaries(&word_list).await;
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
    println!("{}", word_list.word_count());
    println!("{:?}", histo);

    // let words = word_list.get();
    // let ideal = word_list.word_count() >> 5;
    // println!("Ideal: {}", ideal);
    // let mut reduction = words
    //     .iter()
    //     .map(|word| {
    //         (
    //             word,
    //             word.chars()
    //                 .fold(WordList::new(), |list, c| list.whittle(c))
    //                 .word_count(),
    //             calculate_score(&set, word),
    //         )
    //     })
    //     .collect::<Vec<_>>();
    // reduction.sort_by_key(|x| (x.1, (1 << 32) - x.2, x.0));
    // reduction.truncate(50);
    // println!("{:?}", reduction);

    // let words = word_list.get();
    // let mut scores = words
    //     .iter()
    //     .map(|word| (word, calculate_score(&set, word)))
    //     .collect::<Vec<_>>();
    // scores.sort_by_key(|t| t.1);
    // scores.reverse();
    // scores.truncate(20);
    // println!("{:?}", scores);

    let mut game = Game::new("tests".to_string());
    print_results(&game.check("sssss"))?;
    print_results(&game.check("totes"))?;
    print_results(&game.check("tests"))?;
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
