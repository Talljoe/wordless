use std::{convert::TryInto, iter::FromIterator};

#[derive(Clone, Debug)]
pub struct Game {
    guesses: Vec<Vec<LetterResult>>,
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

        // let letters: Vec<LetterResult> = guess
        //     .chars()
        //     .enumerate()
        //     .map(|(i, c)| match word_chars.get_mut(i) {
        //         Some(fc) if *fc == c => {
        //             *fc = '_';
        //             (i, c, Some(LetterResult::Exact(c)))
        //         }
        //         _ => (i, c, None),
        //     })
        //     .map(|(i, c, prev)| {
        //         prev.unwrap_or_else(|| {
        //             if let Ok(found_at) = word_chars.binary_search(&c) {
        //                 word_chars.remove(found_at);
        //                 LetterResult::Contains(c)
        //             } else {
        //                 LetterResult::NotFound(c)
        //             }
        //         })
        //     })
        //     .collect();
        self.guesses.push(letters.clone());
        let result = if letters
            .iter()
            .all(|lr| matches!(lr, LetterResult::Exact(_)))
        {
            GuessResult::Correct
        } else {
            GuessResult::Incorrect
        };

        CheckData {
            letters,
            result,
            guesses: self.guesses.len().try_into().unwrap(),
        }
    }
}
