use std::{convert::TryInto, iter::FromIterator};

#[derive(Clone, Debug)]
pub struct Game {
    guesses: Vec<CheckData>,
    word: String,
}

#[derive(Clone, Debug)]
pub enum LetterResult {
    Exact(char),
    Contains(char),
    NotFound(char),
}

#[derive(Clone, Debug)]
pub enum GuessResult {
    Correct,
    Incorrect,
}

#[derive(Clone, Debug)]
pub struct CheckData {
    pub letters: Vec<LetterResult>,
    pub result: GuessResult,
    pub guesses: u8,
}

impl Game {
    pub fn new(word: String) -> Self {
        Game {
            guesses: vec![],
            word,
        }
    }

    pub fn check(&mut self, guess: &str) -> CheckData {
        if self.guesses.len() == 6 {
            return self.guesses.last().unwrap().clone();
        }

        let mut word_chars = Vec::from_iter(self.word.chars());
        let mut letters: Vec<LetterResult> = vec![];

        for (i, c) in guess.to_ascii_lowercase().chars().enumerate() {
            letters.push(match word_chars.get_mut(i) {
                Some(fc) if *fc == c => {
                    *fc = '_';
                    LetterResult::Exact(c)
                }
                _ => LetterResult::NotFound(c),
            });
        }

        for lr in letters.iter_mut() {
            if let LetterResult::NotFound(c) = lr {
                if let Ok(found_at) = word_chars.binary_search(&c) {
                    word_chars.remove(found_at);
                    *lr = LetterResult::Contains(*c)
                }
            }
        }

        let result = if letters
            .iter()
            .all(|lr| matches!(lr, LetterResult::Exact(_)))
        {
            GuessResult::Correct
        } else {
            GuessResult::Incorrect
        };

        let guesses = self.guesses.len() + 1;
        let verdict = CheckData {
            letters,
            result,
            guesses: guesses.try_into().unwrap(),
        };
        self.guesses.push(verdict.clone());
        verdict
    }
}
