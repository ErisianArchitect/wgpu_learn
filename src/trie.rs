use std::{borrow::Borrow, collections::HashMap, hash::Hash, ops::Deref};

pub struct Trie<T: Copy + Eq + Hash> {
    prefixes: Option<HashMap<T, Trie<T>>>,
    is_leaf: bool,
}

trait TrieWord<T: Copy + Eq + Hash> {
    fn iter(self) -> impl Iterator<Item = T>;
}

impl<T: Copy + Eq + Hash> Trie<T> {
    pub fn new() -> Self {
        Self {
            prefixes: None,
            is_leaf: false,
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.is_leaf
    }

    pub fn insert<It: IntoIterator<Item = T>>(&mut self, word: It) {
        let mut letters = word.into_iter();
        let last = letters.fold(self, |trie, letter| {
            let prefixes = trie.prefixes.get_or_insert_with(|| HashMap::new());
            prefixes.entry(letter).or_insert(Trie::new())
        });
        last.is_leaf = true;
    }

    pub fn contains<I: Borrow<T>, It: IntoIterator<Item = I>>(&self, word: It) -> bool {
        let last = word.into_iter().try_fold(self, |trie, letter| {
            let Some(prefixes) = &trie.prefixes else {
                return None;
            };
            prefixes.get(letter.borrow())
        });
        last.is_some_and(Trie::is_leaf)
    }

    pub fn contains_partial<I: Borrow<T>, It: IntoIterator<Item = I>>(&self, word: It) -> bool {
        let last = word.into_iter().try_fold(self, |trie, letter| {
            let Some(prefixes) = &trie.prefixes else {
                return None;
            };
            prefixes.get(letter.borrow())
        });
        last.is_some()
    }
}

#[test]
fn trie_test() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum Button {
        Up, Down, Left, Right,
        A, B, C, D,
        L1, L2, R1, R2,
        Start, Select,
    }
    use Button::*;
    let mut trie = Trie::<char>::new();
    println!("Result: {}", trie.contains_partial("Forg".chars()));
    let mut trie = Trie::<Button>::new();
    const KONAMI_CODE: [Button; 11] = [Up, Up, Down, Down, Left, Right, Left, Right, B, A, Start];
    const OTHER_CODE: [Button; 12] = [Up, Up, Down, Down, Left, Right, Left, Right, B, A, Start, Select];
    trie.insert(KONAMI_CODE);
    trie.insert(OTHER_CODE);
    let result = trie.contains(KONAMI_CODE);
    let other = trie.contains(OTHER_CODE);
    let part = trie.contains_partial([Up, Up, Down, Down]);
    println!("{} {} {part}", result, other);

}