mod word_list;

use std::collections::{HashMap, HashSet};

use word_list::WordList;

#[derive(Clone, Debug)]
struct DictionarySet {
    position_maps: [HashMap<char, Vec<&'static str>>; 5],
    contains_map: HashMap<char, HashSet<&'static str>>,
}

impl DictionarySet {
    pub fn new(word_count: usize) -> Self {
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

async fn build_dictionaries(word_list: WordList) -> DictionarySet {
    let initial = DictionarySet::new(word_list.word_count());
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

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let word_list = WordList::new();
    let set = build_dictionaries(word_list).await;
    let histo: Vec<_> = set
        .position_maps
        .iter()
        .enumerate()
        .map(|(i, m)| {
            (
                i,
                m.iter()
                    .map(|(letter, list)| (letter, list.len()))
                    .collect::<HashMap<_, _>>(),
            )
        })
        .collect();

    let mut histo: Vec<_> = set.contains_map.iter().map(|(c, m)| (c, m.len())).collect();
    histo.sort_by_key(|item| item.1);
    println!("{:?}", histo);
}
