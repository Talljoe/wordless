use chrono::prelude::*;
use std::{collections::BTreeSet, convert::TryInto, iter::FromIterator};

use crate::word_list::WordList;

#[derive(Clone, Debug)]
pub struct Game {
    guesses: Vec<CheckData>,
    word: String,
    day: Option<usize>,
    hard: bool,
    revealed: BTreeSet<char>,
}

#[derive(Clone, Debug)]
pub enum LetterResult {
    Exact(char),
    Contains(char),
    NotFound(char),
}

impl LetterResult {
    pub fn is_found(&self) -> bool {
        match self {
            LetterResult::Exact(_) => true,
            LetterResult::Contains(_) => true,
            LetterResult::NotFound(_) => false,
        }
    }

    pub fn to_char(&self) -> char {
        match self {
            LetterResult::Exact(c) | LetterResult::Contains(c) | LetterResult::NotFound(c) => *c,
        }
    }
}

#[derive(Clone, Debug)]
pub enum GuessResult {
    Win,
    Incorrect,
    Lose,
    Invalid(String),
}

#[derive(Clone, Debug)]
pub struct CheckData {
    pub letters: Vec<LetterResult>,
    pub result: GuessResult,
    pub guesses: u8,
}

impl Game {
    pub fn for_word(word: &str) -> Self {
        Game {
            guesses: vec![],
            revealed: BTreeSet::new(),
            word: word.to_string(),
            day: None,
            hard: false,
        }
    }

    pub fn new(day: Option<usize>) -> Self {
        let day = day.or_else(|| {
            let wepoch = Local.ymd(2021, 6, 19);
            Local::today()
                .signed_duration_since::<Local>(wepoch)
                .num_days()
                .abs()
                .try_into()
                .ok()
        });

        let word = day.and_then(WordList::get_word_for_day).unwrap();

        Game {
            day,
            ..Game::for_word(word)
        }
    }

    pub fn set_hard_mode(self) -> Self {
        Game { hard: true, ..self }
    }

    pub fn day(&self) -> Option<usize> {
        self.day
    }

    pub fn is_easy(&self) -> bool {
        !self.hard
    }

    pub fn guesses(&self) -> Vec<CheckData> {
        self.guesses.clone()
    }

    pub fn word(&self) -> String {
        self.word.clone()
    }

    pub fn check(&mut self, guess: &str) -> CheckData {
        if let Some(last_guess) = self.guesses.last() {
            if let GuessResult::Lose = last_guess.result {
                return last_guess.clone();
            }
        }

        if self.hard {
            for revealed in self.revealed.iter() {
                if !guess.contains(*revealed) {
                    return CheckData {
                        letters: guess.chars().map(|c| LetterResult::NotFound(c)).collect(),
                        result: GuessResult::Invalid(guess.to_string()),
                        guesses: 0,
                    };
                }
            }
        }

        let mut word_chars = Vec::from_iter(self.word.chars());
        let mut letters: Vec<LetterResult> = vec![];

        for (i, c) in guess.to_ascii_lowercase().chars().enumerate() {
            letters.push(match word_chars.get_mut(i) {
                Some(fc) if *fc == c => {
                    *fc = '_';
                    self.revealed.insert(c);
                    LetterResult::Exact(c)
                }
                _ => LetterResult::NotFound(c),
            });
        }

        word_chars.sort();

        for lr in letters.iter_mut() {
            if let LetterResult::NotFound(c) = lr {
                if let Ok(found_at) = word_chars.binary_search(&c) {
                    word_chars.remove(found_at);
                    self.revealed.insert(*c);
                    *lr = LetterResult::Contains(*c)
                }
            }
        }

        let guesses = self.guesses.len() + 1;

        let correct = letters
            .iter()
            .all(|lr| matches!(lr, LetterResult::Exact(_)));

        let result = match guesses {
            1..=5 if correct => GuessResult::Win,
            1..=5 => GuessResult::Incorrect,
            _ => GuessResult::Lose,
        };

        let verdict = CheckData {
            letters,
            result,
            guesses: guesses.try_into().unwrap(),
        };
        self.guesses.push(verdict.clone());
        verdict
    }
}
