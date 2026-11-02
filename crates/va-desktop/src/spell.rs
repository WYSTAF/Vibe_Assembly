//! Prompt spellcheck: curated fixes plus a dictionary-backed typo detector.
//!
//! The curated table catches ambiguous misspellings exactly. Anything else is
//! compared against an embedded dictionary, and lowercase words that are one
//! edit away from a common word are underlined. Words inside backticks, paths,
//! URLs, handles and identifiers are never touched.

use std::cmp::Ordering;
use std::collections::HashSet;
use std::ops::Range;
use std::sync::OnceLock;

const DICTIONARY: &str = include_str!("../assets/dictionary.txt");
const BANDS: &str = include_str!("../assets/bands.txt");
const VOCAB: &str = include_str!("../assets/vocab.txt");

/// Below this score a dictionary match is too weak to show.
const MIN_SCORE: u32 = 1150;
const VOCAB_TIER: u32 = 230;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub range: Range<usize>,
    pub original: String,
    pub replacement: String,
}

struct Word {
    range: Range<usize>,
    lower: String,
}

/// Every typo in `text`, curated or dictionary-backed.
pub fn suggestions(text: &str) -> Vec<Suggestion> {
    detect(text, true)
}

fn detect(text: &str, general: bool) -> Vec<Suggestion> {
    words(text)
        .into_iter()
        .filter_map(|word| {
            let original = &text[word.range.clone()];
            let replacement = if let Some(fix) = correction(&word.lower) {
                apply_case(original, fix)
            } else if general && suspect(original, &word.lower) {
                let lexicon = lexicon();
                if lexicon.is_known(&word.lower) {
                    return None;
                }
                apply_case(original, &lexicon.best(&word.lower)?)
            } else {
                return None;
            };
            Some(Suggestion {
                original: original.to_string(),
                replacement,
                range: word.range,
            })
        })
        .collect()
}

/// The typo closest to the caret — the one it is in, or the nearest across
/// the sentence, so Tab can work through a prompt one fix at a time.
pub fn nearest(text: &str, caret: usize) -> Option<Suggestion> {
    let caret = caret.min(text.len());
    suggestions(text).into_iter().min_by_key(|fix| {
        if fix.range.contains(&caret) || fix.range.end == caret {
            0
        } else if caret < fix.range.start {
            fix.range.start - caret
        } else {
            caret - fix.range.end
        }
    })
}

/// Rewrites every curated misspelling on send. Dictionary suggestions stay
/// opt-in so names and jargon are never rewritten automatically.
pub fn correct(text: &str) -> (String, usize) {
    let fixes = detect(text, false);
    if fixes.is_empty() {
        return (text.to_string(), 0);
    }
    let mut out = text.to_string();
    for fix in fixes.iter().rev() {
        out.replace_range(fix.range.clone(), &fix.replacement);
    }
    (out, fixes.len())
}

fn lexicon() -> &'static Lexicon {
    static LEXICON: OnceLock<Lexicon> = OnceLock::new();
    LEXICON.get_or_init(Lexicon::load)
}

struct Lexicon {
    dictionary: WordList,
    bands: Bands,
    vocab: HashSet<&'static str>,
}

impl Lexicon {
    fn load() -> Self {
        Self {
            dictionary: WordList::new(DICTIONARY),
            bands: Bands::new(BANDS),
            vocab: VOCAB.lines().collect(),
        }
    }

    fn is_known(&self, word: &str) -> bool {
        self.dictionary.index_of(word).is_some()
            || self.bands.index_of(word).is_some()
            || self.vocab.contains(word)
    }

    fn best(&self, word: &str) -> Option<String> {
        let bytes = word.as_bytes();
        let mut best = None;
        let mut candidate = String::with_capacity(word.len() + 1);

        // Insertions score extra: every letter the writer typed is preserved in order.
        for position in 0..=bytes.len() {
            for letter in b'a'..=b'z' {
                candidate.clear();
                candidate.push_str(&word[..position]);
                candidate.push(letter as char);
                candidate.push_str(&word[position..]);
                self.consider(word, &candidate, 1000, &mut best);
            }
        }

        for position in 0..bytes.len() {
            candidate.clear();
            candidate.push_str(&word[..position]);
            candidate.push_str(&word[position + 1..]);
            // A doubled letter is usually a slip, not a word the writer meant.
            let doubled = (position > 0 && bytes[position] == bytes[position - 1])
                || (position + 1 < bytes.len() && bytes[position] == bytes[position + 1]);
            let bonus = if doubled { 180 } else { 0 };
            self.consider(word, &candidate, 900 + bonus, &mut best);
        }

        for position in 0..bytes.len() {
            for letter in b'a'..=b'z' {
                if letter == bytes[position] {
                    continue;
                }
                candidate.clear();
                candidate.push_str(&word[..position]);
                candidate.push(letter as char);
                candidate.push_str(&word[position + 1..]);
                let nearby = if adjacent_keys(bytes[position], letter) {
                    200
                } else {
                    0
                };
                self.consider(word, &candidate, 600 + nearby, &mut best);
            }
        }

        for position in 0..bytes.len().saturating_sub(1) {
            if bytes[position] == bytes[position + 1] {
                continue;
            }
            candidate.clear();
            candidate.push_str(&word[..position]);
            candidate.push(bytes[position + 1] as char);
            candidate.push(bytes[position] as char);
            candidate.push_str(&word[position + 2..]);
            self.consider(word, &candidate, 1200, &mut best);
        }

        best.map(|(_, word)| word)
    }

    fn consider(&self, word: &str, candidate: &str, base: u32, best: &mut Option<(u32, String)>) {
        let Some(tier) = self.tier(candidate) else {
            return;
        };
        let (prefix, suffix) = common_affixes(word, candidate);
        let length_bonus = if word.len() == candidate.len() {
            80
        } else {
            60
        };
        let score = base + prefix as u32 * 60 + suffix as u32 * 100 + length_bonus + tier;
        if score < MIN_SCORE {
            return;
        }
        let better = match best {
            Some((top, top_word)) => {
                score > *top || (score == *top && candidate < top_word.as_str())
            }
            None => true,
        };
        if better {
            *best = Some((score, candidate.to_string()));
        }
    }

    fn tier(&self, candidate: &str) -> Option<u32> {
        let band = self.bands.tier(candidate);
        let vocab = self.vocab.contains(candidate).then_some(VOCAB_TIER);
        match (band, vocab) {
            (Some(band), Some(vocab)) => Some(band.max(vocab)),
            (Some(band), None) => Some(band),
            (None, Some(vocab)) => Some(vocab),
            (None, None) => None,
        }
    }
}

fn suspect(original: &str, lower: &str) -> bool {
    if original.len() < 3 || !original.bytes().all(|letter| letter.is_ascii_lowercase()) {
        return false;
    }
    // Three consonants are usually an acronym or identifier, not prose.
    lower.len() != 3
        || lower
            .bytes()
            .any(|letter| matches!(letter, b'a' | b'e' | b'i' | b'o' | b'u' | b'y'))
}

fn common_affixes(word: &str, candidate: &str) -> (usize, usize) {
    let word = word.as_bytes();
    let candidate = candidate.as_bytes();
    let mut prefix = 0;
    while prefix < word.len().min(candidate.len()) && word[prefix] == candidate[prefix] {
        prefix += 1;
    }
    let mut suffix = 0;
    let limit = word.len().min(candidate.len()) - prefix;
    while suffix < limit && word[word.len() - 1 - suffix] == candidate[candidate.len() - 1 - suffix]
    {
        suffix += 1;
    }
    (prefix, suffix)
}

fn adjacent_keys(first: u8, second: u8) -> bool {
    const ROWS: [&[u8]; 3] = [b"qwertyuiop", b"asdfghjkl", b"zxcvbnm"];
    let position = |key: u8| {
        ROWS.iter().enumerate().find_map(|(row, keys)| {
            keys.iter()
                .position(|&slot| slot == key)
                .map(|column| (row as i32, column as i32))
        })
    };
    let (Some((first_row, first_column)), Some((second_row, second_column))) =
        (position(first), position(second))
    else {
        return false;
    };
    let rows = (first_row - second_row).abs();
    let columns = (first_column - second_column).abs();
    (rows == 0 && columns == 1) || (rows == 1 && columns <= 1)
}

/// Newline-separated words sorted by byte order.
struct WordList {
    text: &'static str,
    ends: Vec<u32>,
}

impl WordList {
    fn new(text: &'static str) -> Self {
        let mut ends = Vec::with_capacity(text.len() / 8);
        let mut offset = 0;
        for line in text.lines() {
            offset += line.len();
            ends.push(offset as u32);
            offset += 1;
        }
        Self { text, ends }
    }

    fn index_of(&self, word: &str) -> Option<usize> {
        let (mut low, mut high) = (0, self.ends.len());
        while low < high {
            let middle = (low + high) / 2;
            match self.line(middle).cmp(word) {
                Ordering::Less => low = middle + 1,
                Ordering::Greater => high = middle,
                Ordering::Equal => return Some(middle),
            }
        }
        None
    }

    fn line(&self, index: usize) -> &'static str {
        let start = match index {
            0 => 0,
            _ => self.ends[index - 1] as usize + 1,
        };
        &self.text[start..self.ends[index] as usize]
    }
}

/// A word list with a SCOWL commonness tier per line (`word 10`).
struct Bands {
    words: WordList,
}

impl Bands {
    fn new(text: &'static str) -> Self {
        Self {
            words: WordList::new(text),
        }
    }

    fn index_of(&self, word: &str) -> Option<usize> {
        self.words.index_of_word(word)
    }

    fn tier(&self, word: &str) -> Option<u32> {
        let index = self.index_of(word)?;
        let band = self.words.line(index).rsplit_once(' ')?.1.parse().ok()?;
        match band {
            10 => Some(260),
            20 => Some(230),
            35 => Some(200),
            40 => Some(165),
            50 => Some(130),
            55 => Some(110),
            60 => Some(95),
            70 => Some(80),
            _ => None,
        }
    }
}

impl WordList {
    fn index_of_word(&self, word: &str) -> Option<usize> {
        let (mut low, mut high) = (0, self.ends.len());
        while low < high {
            let middle = (low + high) / 2;
            let line = self.line(middle);
            let candidate = line.rsplit_once(' ').map_or(line, |(word, _)| word);
            match candidate.cmp(word) {
                Ordering::Less => low = middle + 1,
                Ordering::Greater => high = middle,
                Ordering::Equal => return Some(middle),
            }
        }
        None
    }
}

fn words(text: &str) -> Vec<Word> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut backticks = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            backticks += 1;
            i += 1;
        } else if bytes[i].is_ascii_alphabetic() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            if backticks.is_multiple_of(2) && prose_boundary(text, start, i) {
                out.push(Word {
                    range: start..i,
                    lower: text[start..i].to_ascii_lowercase(),
                });
            }
        } else {
            i += text[i..].chars().next().map_or(1, |ch| ch.len_utf8());
        }
    }
    out
}

fn prose_boundary(text: &str, start: usize, end: usize) -> bool {
    // Skip identifiers, paths, domains, handles and contractions' fragments.
    const BEFORE: &[char] = &['_', '/', '\\', '.', ':', '@', '#', '$', '-', '\''];
    const AFTER: &[char] = &['_', '/', '\\', ':', '@', '#', '$', '-', '\''];
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    !before.is_some_and(|ch| ch.is_alphanumeric() || BEFORE.contains(&ch))
        && !after.is_some_and(|ch| ch.is_alphanumeric() || AFTER.contains(&ch))
}

fn apply_case(original: &str, replacement: &str) -> String {
    if original.chars().all(|ch| ch.is_ascii_uppercase()) {
        return replacement.to_ascii_uppercase();
    }
    if original
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        let mut chars = replacement.chars();
        let mut out: String = chars
            .next()
            .map(|first| first.to_uppercase().collect())
            .unwrap_or_default();
        out.push_str(chars.as_str());
        return out;
    }
    replacement.to_string()
}

fn correction(word: &str) -> Option<&'static str> {
    Some(match word {
        // Contractions and informal joins
        "alot" => "a lot",
        "arent" => "aren't",
        "aswell" => "as well",
        "cant" => "can't",
        "couldnt" => "couldn't",
        "couldve" => "could've",
        "didnt" => "didn't",
        "doesnt" => "doesn't",
        "dont" => "don't",
        "eachother" => "each other",
        "hadnt" => "hadn't",
        "hasnt" => "hasn't",
        "havent" => "haven't",
        "heres" => "here's",
        "hows" => "how's",
        "incase" => "in case",
        "infront" => "in front",
        "isnt" => "isn't",
        "ive" => "I've",
        "mightve" => "might've",
        "mustnt" => "mustn't",
        "mustve" => "must've",
        "noone" => "no one",
        "shouldnt" => "shouldn't",
        "shouldve" => "should've",
        "thats" => "that's",
        "theres" => "there's",
        "theyre" => "they're",
        "theyve" => "they've",
        "wasnt" => "wasn't",
        "weve" => "we've",
        "whats" => "what's",
        "wheres" => "where's",
        "wont" => "won't",
        "wouldnt" => "wouldn't",
        "wouldve" => "would've",
        "youre" => "you're",
        "youve" => "you've",

        // Common English misspellings
        "accross" => "across",
        "accomodate" => "accommodate",
        "acheive" => "achieve",
        "acheived" => "achieved",
        "acheivement" => "achievement",
        "acommodate" => "accommodate",
        "adress" => "address",
        "adresses" => "addresses",
        "aditional" => "additional",
        "anwser" => "answer",
        "appearence" => "appearance",
        "aproach" => "approach",
        "apropriate" => "appropriate",
        "assigment" => "assignment",
        "basicly" => "basically",
        "beacuse" => "because",
        "becasue" => "because",
        "becuase" => "because",
        "beggining" => "beginning",
        "begining" => "beginning",
        "beleive" => "believe",
        "beleived" => "believed",
        "beleives" => "believes",
        "calender" => "calendar",
        "catagory" => "category",
        "certian" => "certain",
        "collegue" => "colleague",
        "collegues" => "colleagues",
        "comming" => "coming",
        "commitee" => "committee",
        "completly" => "completely",
        "concious" => "conscious",
        "consistant" => "consistent",
        "diffrent" => "different",
        "diferent" => "different",
        "differnt" => "different",
        "diffcult" => "difficult",
        "enought" => "enough",
        "enviroment" => "environment",
        "experiance" => "experience",
        "familar" => "familiar",
        "finaly" => "finally",
        "foriegn" => "foreign",
        "goverment" => "government",
        "grammer" => "grammar",
        "happend" => "happened",
        "happenned" => "happened",
        "immediatly" => "immediately",
        "independant" => "independent",
        "knowlege" => "knowledge",
        "langauge" => "language",
        "maintainance" => "maintenance",
        "managment" => "management",
        "mesage" => "message",
        "mesages" => "messages",
        "mispell" => "misspell",
        "mispelling" => "misspelling",
        "neccessary" => "necessary",
        "necesary" => "necessary",
        "necessery" => "necessary",
        "occassion" => "occasion",
        "occured" => "occurred",
        "occurence" => "occurrence",
        "occuring" => "occurring",
        "ocurred" => "occurred",
        "ocurring" => "occurring",
        "persistant" => "persistent",
        "posession" => "possession",
        "prefered" => "preferred",
        "priviledge" => "privilege",
        "probaly" => "probably",
        "proffesional" => "professional",
        "questionaire" => "questionnaire",
        "realy" => "really",
        "refered" => "referred",
        "relevent" => "relevant",
        "remeber" => "remember",
        "resistence" => "resistance",
        "rythm" => "rhythm",
        "sentense" => "sentence",
        "sentenses" => "sentences",
        "seperately" => "separately",
        "similiar" => "similar",
        "sincerly" => "sincerely",
        "suprise" => "surprise",
        "tabel" => "table",
        "tommorow" => "tomorrow",
        "tommorrow" => "tomorrow",
        "truely" => "truly",
        "unfortunatly" => "unfortunately",
        "untill" => "until",
        "usualy" => "usually",
        "wierd" => "weird",
        "writting" => "writing",

        // Letter-order slips
        "coudl" => "could",
        "hte" => "the",
        "hwat" => "what",
        "jsut" => "just",
        "shoudl" => "should",
        "taht" => "that",
        "teh" => "the",
        "thier" => "their",
        "thsi" => "this",
        "waht" => "what",
        "whcih" => "which",
        "whta" => "what",
        "wiht" => "with",
        "wich" => "which",
        "woudl" => "would",

        // Software terms
        "acess" => "access",
        "activites" => "activities",
        "actualy" => "actually",
        "algorithim" => "algorithm",
        "algoritm" => "algorithm",
        "appication" => "application",
        "arguement" => "argument",
        "arguements" => "arguments",
        "asychronous" => "asynchronous",
        "asyncronous" => "asynchronous",
        "availble" => "available",
        "avaliable" => "available",
        "booleam" => "boolean",
        "booleen" => "boolean",
        "buidl" => "build",
        "bulid" => "build",
        "calback" => "callback",
        "calcluate" => "calculate",
        "caluclate" => "calculate",
        "charactor" => "character",
        "charater" => "character",
        "chekc" => "check",
        "closue" => "closure",
        "closuer" => "closure",
        "comaptible" => "compatible",
        "commnad" => "command",
        "compilier" => "compiler",
        "compilor" => "compiler",
        "compatable" => "compatible",
        "compoent" => "component",
        "compoents" => "components",
        "componet" => "component",
        "concatinate" => "concatenate",
        "configration" => "configuration",
        "configuaration" => "configuration",
        "configuartion" => "configuration",
        "confg" => "config",
        "conifg" => "config",
        "constuctor" => "constructor",
        "contianer" => "container",
        "contorl" => "control",
        "curent" => "current",
        "currnet" => "current",
        "deafult" => "default",
        "decleration" => "declaration",
        "defalut" => "default",
        "defintion" => "definition",
        "defualt" => "default",
        "dependancies" => "dependencies",
        "dependancy" => "dependency",
        "dependecies" => "dependencies",
        "dependecy" => "dependency",
        "depenency" => "dependency",
        "deployement" => "deployment",
        "deplyoment" => "deployment",
        "depricated" => "deprecated",
        "descision" => "decision",
        "destory" => "destroy",
        "directoy" => "directory",
        "dirctory" => "directory",
        "docuemnt" => "document",
        "documet" => "document",
        "documnet" => "document",
        "embeded" => "embedded",
        "excecute" => "execute",
        "exisiting" => "existing",
        "exisitng" => "existing",
        "exmapel" => "example",
        "exmaple" => "example",
        "exmaples" => "examples",
        "faild" => "failed",
        "featue" => "feature",
        "feautre" => "feature",
        "fialed" => "failed",
        "fucntion" => "function",
        "fucntions" => "functions",
        "funciton" => "function",
        "functoin" => "function",
        "funtion" => "function",
        "genric" => "generic",
        "heigth" => "height",
        "hieght" => "height",
        "identifer" => "identifier",
        "implemenation" => "implementation",
        "implemet" => "implement",
        "implemetation" => "implementation",
        "implementatoin" => "implementation",
        "implment" => "implement",
        "implmentation" => "implementation",
        "initalize" => "initialize",
        "initialze" => "initialize",
        "instanciate" => "instantiate",
        "intialize" => "initialize",
        "interger" => "integer",
        "intreface" => "interface",
        "iterater" => "iterator",
        "iteratior" => "iterator",
        "lenght" => "length",
        "lifetme" => "lifetime",
        "optimze" => "optimize",
        "ouput" => "output",
        "outupt" => "output",
        "overide" => "override",
        "parametr" => "parameter",
        "paramter" => "parameter",
        "paramters" => "parameters",
        "performace" => "performance",
        "perfomance" => "performance",
        "perhpas" => "perhaps",
        "promt" => "prompt",
        "promtp" => "prompt",
        "promtps" => "prompts",
        "propogate" => "propagate",
        "realease" => "release",
        "recieve" => "receive",
        "recieved" => "received",
        "recieveing" => "receiving",
        "reciever" => "receiver",
        "recievers" => "receivers",
        "recieves" => "receives",
        "refacotr" => "refactor",
        "refacter" => "refactor",
        "relese" => "release",
        "reponse" => "response",
        "requried" => "required",
        "requierd" => "required",
        "reslut" => "result",
        "resposne" => "response",
        "resutl" => "result",
        "retrun" => "return",
        "retun" => "return",
        "reuslt" => "result",
        "reutrn" => "return",
        "seach" => "search",
        "sepcify" => "specify",
        "seperated" => "separated",
        "seperator" => "separator",
        "seperators" => "separators",
        "seperate" => "separate",
        "serach" => "search",
        "specfic" => "specific",
        "specifc" => "specific",
        "statment" => "statement",
        "statments" => "statements",
        "stuct" => "struct",
        "sturct" => "struct",
        "succesful" => "successful",
        "successfull" => "successful",
        "sucess" => "success",
        "sucessful" => "successful",
        "synatx" => "syntax",
        "sytnax" => "syntax",
        "tempalte" => "template",
        "templte" => "template",
        "thred" => "thread",
        "upadte" => "update",
        "upate" => "update",
        "updaet" => "update",
        "vairable" => "variable",
        "varaible" => "variable",
        "varible" => "variable",
        "widht" => "width",
        "wokring" => "working",
        "workign" => "working",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(text: &str) -> String {
        suggestions(text)
            .into_iter()
            .map(|suggestion| suggestion.replacement)
            .collect::<Vec<_>>()
            .join("|")
    }

    #[test]
    fn finds_common_and_technical_typos() {
        assert_eq!(fix("teh cat"), "the");
        assert_eq!(fix("recieve the paramters"), "receive|parameters");
        assert_eq!(fix("dont do it"), "don't");
        assert_eq!(fix("is teh functoin working"), "the|function");
    }

    #[test]
    fn catches_ordinary_typos_beyond_the_curated_list() {
        assert_eq!(fix("helo wrld thisisnotaword"), "hello|world");
        assert_eq!(fix("inpt colum scrool"), "input|column|scroll");
        assert_eq!(fix("seperate definately"), "separate|definitely");
    }

    #[test]
    fn prefers_dropping_a_doubled_letter() {
        assert_eq!(fix("dooing"), "doing");
        assert_eq!(fix("helllo"), "hello");
        assert_eq!(fix("whaat"), "what");
    }

    #[test]
    fn keeps_valid_words_and_technical_terms_alone() {
        assert_eq!(fix("hello world form from its"), "");
        assert_eq!(fix("gpui tokio redis tmux winget stdio utf8"), "");
        assert_eq!(fix("function receives message"), "");
    }

    #[test]
    fn general_detection_skips_capitalized_words() {
        assert_eq!(fix("Helo Wrld"), "");
        assert_eq!(fix("Qoder"), "");
    }

    #[test]
    fn curated_corrections_win_over_dictionary_entries() {
        assert_eq!(
            fix("wich wierd untill tabel enviroment"),
            "which|weird|until|table|environment"
        );
    }

    #[test]
    fn send_correction_stays_limited_to_the_curated_list() {
        let (text, fixed) = correct("helo wrld recieve teh");
        assert_eq!(text, "helo wrld receive the");
        assert_eq!(fixed, 2);
    }

    #[test]
    fn preserves_case_and_contraction_capitalization() {
        assert_eq!(fix("Teh"), "The");
        assert_eq!(fix("TEH"), "THE");
        assert_eq!(fix("Dont"), "Don't");
        assert_eq!(fix("DONT"), "DON'T");
        assert_eq!(fix("alot"), "a lot");
        assert_eq!(fix("Alot"), "A lot");
        assert_eq!(fix("ive"), "I've");
    }

    #[test]
    fn leaves_code_urls_paths_handles_and_unicode_alone() {
        assert_eq!(fix("run `recieve` now"), "");
        assert_eq!(fix("```\nrecieve\n```"), "");
        assert_eq!(fix("see https://exmaple.com/teh"), "");
        assert_eq!(fix("open docs/recieve.rs"), "");
        assert_eq!(fix("@teh said so"), "");
        assert_eq!(fix("snake_teh_case"), "");
        assert_eq!(fix("café teh"), "the");
        assert_eq!(fix("سلام teh"), "the");
    }

    #[test]
    fn nearest_prefers_the_typo_at_the_caret_then_the_closest_one() {
        let text = "teh cat";
        assert_eq!(nearest(text, 0).unwrap().replacement, "the");
        assert_eq!(nearest(text, 3).unwrap().replacement, "the");
        assert_eq!(nearest(text, 6).unwrap().replacement, "the");
        assert_eq!(nearest("teh recieve", 11).unwrap().replacement, "receive");
        assert_eq!(nearest("teh ", 4).unwrap().range, 0..3);
        assert!(nearest("", 0).is_none());
        assert!(nearest("all correct here", 5).is_none());
    }

    #[test]
    fn correct_rewrites_every_match_and_counts_them() {
        let (text, fixed) = correct("teh functoin recieves teh data");
        assert_eq!(text, "the function receives the data");
        assert_eq!(fixed, 4);
        let (same, none) = correct("nothing to see here");
        assert_eq!(same, "nothing to see here");
        assert_eq!(none, 0);
    }
}
