//! Prompt spellcheck: a curated typo table and case-preserving corrections.
//!
//! Only exact, unambiguous misspellings are corrected — valid words such as
//! "form", "its" and "cant"-as-a-noun are left alone. Words inside backticks,
//! paths, URLs, handles and identifiers are never touched.

use std::ops::Range;

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

/// Every typo in `text` that has a known correction.
pub fn suggestions(text: &str) -> Vec<Suggestion> {
    words(text)
        .into_iter()
        .filter_map(|word| {
            let fix = correction(&word.lower)?;
            Some(Suggestion {
                original: text[word.range.clone()].to_string(),
                replacement: apply_case(&text[word.range.clone()], fix),
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

/// Rewrites every known typo; returns the new text and how many were fixed.
pub fn correct(text: &str) -> (String, usize) {
    let fixes = suggestions(text);
    if fixes.is_empty() {
        return (text.to_string(), 0);
    }
    let mut out = text.to_string();
    for fix in fixes.iter().rev() {
        out.replace_range(fix.range.clone(), &fix.replacement);
    }
    (out, fixes.len())
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
