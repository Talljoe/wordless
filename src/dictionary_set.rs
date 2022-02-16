use std::collections::{HashMap, HashSet};

use crate::word_list::WordList;

#[derive(Clone, Debug)]
pub struct DictionarySet {
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

    pub fn from_word_list(word_list: &WordList) -> Self {
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

    pub fn list_for_position(&self, index: usize) -> HashMap<char, HashSet<&'static str>> {
        self.position_maps[index].clone()
    }
}
