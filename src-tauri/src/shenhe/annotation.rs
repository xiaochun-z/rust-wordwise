use super::types::{Annotator, Clean, Cleaner, DictRecord};
use csv::{Reader, ReaderBuilder};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Error, ErrorKind};
use std::result::Result;

const WORDWISE_DICTIONARY_PATH: &str = "resources/wordwise-dict.";
const LEMMA_DICTIONARY_PATH: &str = "resources/lemmatization-en.csv";

pub fn load_dict(lang: &str) -> Result<HashMap<String, DictRecord>, Error> {
    let wordwise_dict_path = format!("{}{}.csv", WORDWISE_DICTIONARY_PATH, lang);
    let file = match File::open(&wordwise_dict_path) {
        Ok(file) => file,
        Err(e) => {
            println!("{:?}", e);
            if e.kind() == ErrorKind::NotFound {
                println!("{} not found", wordwise_dict_path);
            }
            return Err(e);
        }
    };

    let mut reader = Reader::from_reader(file);

    let mut wordwise_dict: HashMap<String, DictRecord> = HashMap::new();

    for result in reader.records() {
        let record = result?;
        let (word, phoneme, full_def, short_def, example_sentences, hint_lvl) = (
            record.get(1).unwrap().to_string(),
            record.get(2).unwrap().to_string(),
            record.get(3).unwrap().to_string(),
            record.get(4).unwrap().to_string(),
            record.get(5).unwrap().to_string(),
            record.get(6).unwrap().parse::<i32>().unwrap(),
        );
        wordwise_dict.insert(
            word.clone(),
            DictRecord {
                word: word.clone(),
                phoneme,
                full_def,
                short_def,
                example_sentences,
                hint_lvl,
            },
        );
    }
    //println!("{:?}", wordwise_dict.get("amperage"));

    Ok(wordwise_dict)
}

pub fn load_lemma() -> Result<HashMap<String, String>, Error> {
    let file = File::open(LEMMA_DICTIONARY_PATH)?;
    let mut reader = ReaderBuilder::new().has_headers(true).from_reader(file);
    let lemma_dict: HashMap<_, _> = reader
        .records()
        .map(|result| {
            result.map(|record| {
                (
                    record.get(1).unwrap().to_string(),
                    record.get(0).unwrap().to_string(),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect();

    Ok(lemma_dict)
}

pub fn annotate_phrase(
    annotator: &dyn Annotator,
    sentence: &str,
    dict: &HashMap<String, DictRecord>,
    lemma_dict: &HashMap<String, String>,
    def_length: i32,
    including_phoneme: bool,
    hint_level: i32,
) -> String {
    let words: Vec<&str> = sentence.split_whitespace().collect();
    let mut result = String::new();
    let mut i = 0;
    let max_phrase_length = 5;

    while i < words.len() {
        let mut longest_match = None;
        let mut longest_length = 0;

        // Try to find the longest phrase in the dictionary, up to max_phrase_length words
        for j in (i + 1)..=words.len() {
            if j - i > max_phrase_length {
                break;
            }
            let phrase: String = words[i..j].join(" ");
            let (cleaned_phrase, _, _) = Cleaner::clean_word(&phrase, false);

            if dict.contains_key(&cleaned_phrase) {
                let length = j - i;
                if length > longest_length {
                    longest_length = length;
                    longest_match = Some(phrase);
                }
            }
        }

        if let Some(phrase) = longest_match {
            //println!("{} -> {}", phrase, longest_length);
            let dict_record = get_dict_record(phrase.as_str(), dict, lemma_dict, hint_level);
            match dict_record {
                Some(dr) => {
                    result.push_str(&format!(
                        "{} ",
                        annotator.annotate(dr, phrase.as_str(), def_length, including_phoneme)
                    ));
                }
                None => {
                    result.push_str(&format!("{} ", phrase.as_str()));
                }
            }
            i += longest_length;
        } else {
            // If no phrase matches, check for individual word match
            let dict_record = get_dict_record(words[i], dict, lemma_dict, hint_level);

            match dict_record {
                Some(dr) => {
                    result.push_str(&format!(
                        "{} ",
                        annotator.annotate(dr, words[i], def_length, including_phoneme)
                    ));
                }
                None => {
                    result.push_str(&format!("{} ", words[i]));
                }
            }
            i += 1;
        }
    }

    result.trim_end().to_string()
}

fn get_dict_record<'a>(
    word: &str,
    wordwise_dict: &'a HashMap<String, DictRecord>,
    lemma_dict: &HashMap<String, String>,
    hint_level: i32,
) -> Option<&'a DictRecord> {
    let (clean_word, _, _) = Cleaner::clean_word(word, true);
    //println!("{} -> {}, {:?}", word, clean_word, wordwise_dict.get(clean_word.as_str()));
    if let Some(dict_record) = wordwise_dict.get(clean_word.as_str()) {
        // Skip the word if hint level is not met
        if dict_record.hint_lvl >= hint_level {
            return Some(dict_record);
        }
        return None;
    }

    // Not found, and it's not a phrase, find its normal form
    if !word.contains(' ') {
        if let Some(normal_form) = lemma_dict.get(clean_word.as_str()) {
            // Then, find the normal form word in the wordwise dictionary
            if let Some(dict_record) = wordwise_dict.get(normal_form) {
                // Skip the word if hint level is not met
                if dict_record.hint_lvl >= hint_level {
                    return Some(dict_record);
                }

                return None;
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::super::types::*;
    use super::*;

    #[test]
    fn test_load_dict() {
        let lang = "en";
        let result = load_dict(lang);

        match result {
            Ok(dict) => {
                assert!(!dict.is_empty());
                assert!(dict.get("amperage").is_some());
            }
            Err(_) => {
                // Add your assertions for error cases
                panic!("Failed to load dictionary");
            }
        }
    }

    #[test]
    fn test_load_lemma() {
        let result = load_lemma();
        match result {
            Ok(dict) => {
                assert!(!dict.is_empty());
                //println!("{:?}", dict);
                assert_eq!(dict.get("zips").unwrap(), "zip");
            }
            Err(_) => {
                panic!("Failed to load lemmatization dictionary");
            }
        }
    }

    #[test]
    fn test_clean_word() {
        let test_cases = vec![
            (
                ", Hello, World，大家！!*•-&",
                true,
                "hello, world，大家！",
                ", ",
                "!*•-&",
            ),
            (
                "Hello, World，大家！!*•-&",
                false,
                "Hello, World，大家！",
                "",
                "!*•-&",
            ),
        ];
        for (word, lowercase, expected_cleaned_word, expected_prefix, expected_suffix) in test_cases
        {
            let (cleaned_word, prefix, suffix) = Cleaner::clean_word(word, lowercase);
            assert_eq!(cleaned_word, expected_cleaned_word);
            assert_eq!(prefix, expected_prefix);
            assert_eq!(suffix, expected_suffix);
        }
    }

    #[test]
    fn test_get_dict_record() {
        let word = "riboses";
        let wordwise_dict = load_dict("en").unwrap();
        let lemma_dict = load_lemma().unwrap();
        let result = get_dict_record(word, &wordwise_dict, &lemma_dict, 1);
        assert!(result.is_some());
        match result {
            Some(dict_record) => {
                //println!("{:?}", dict_record);
                assert_eq!(dict_record.word, "ribose");
            }
            None => {
                panic!("Failed to find word in dictionary");
            }
        }
    }

    #[test]
    fn test_get_meaning() {
        let word = "pictorial";
        let dict = load_dict("en").unwrap();
        let dict_record = dict.get(word).unwrap();
        assert_eq!(dict_record.phoneme, "/pɪkˈtɔriəl/");
        assert_eq!(
            dict_record.full_def,
            "of or relating to painting or drawing"
        );
        let res = dict_record.get_meaning(2, true);
        assert_eq!(res, "/pɪkˈtɔriəl/ of or relating to painting or drawing");
    }

    #[test]
    fn test_wrap_with_ruby_tag() {
        let word = "pictorials.";
        let dict = load_dict("en").unwrap();
        let dict_record = dict.get("pictorial").unwrap();
        let annotator: Box<dyn Annotator> = Box::new(RubyAnnotator {});
        let res = annotator.annotate(&dict_record, word, 2, true);
        assert_eq!(
            res,
            "<ruby>pictorials<rt>/pɪkˈtɔriəl/ of or relating to painting or drawing</rt></ruby>."
        );
        let res = annotator.annotate(&dict_record, word, 1, false);
        assert_eq!(
            res,
            "<ruby>pictorials<rt>relating to a drawing</rt></ruby>."
        );
    }

    #[test]
    fn test_annotate_phrase() {
        let data = vec![("I think this is in someone's pocket, but I'm not ascertained.","I think this is <ruby>in someone's pocket<rt>under someone's control</rt></ruby>, but I'm not <ruby>ascertained<rt>discovered by a method</rt></ruby>.", 1),
        ("I think this is in someone's pocket but I'm not ascertained","I think this is <ruby>in someone's pocket<rt>under someone's control</rt></ruby> but I'm not <ruby>ascertained<rt>discovered by a method</rt></ruby>", 1),
        ("I think this is in someone's pocket but I'm not ascertained.","I think this is in someone's pocket but I'm not ascertained.", 2),
        ("The business of eating being concluded, and no one uttering a word of sociable conversation, I approached a window to examine the weather.","The business of eating being concluded, and no one <ruby>uttering<rt>complete and total</rt></ruby> a word of <ruby>sociable<rt>involving friendly relations</rt></ruby> conversation, I approached a window to examine the weather.",1),
        ("unreasonable versatile.","<ruby>unreasonable<rt>not fair or appropriate</rt></ruby> <ruby>versatile<rt>able to do different things</rt></ruby>.", 3),
        ("unreasonable versatile.","unreasonable <ruby>versatile<rt>able to do different things</rt></ruby>.", 4),
        ("two <span>unreasonable</span> versatile one.","two <span>unreasonable</span> <ruby>versatile<rt>able to do different things</rt></ruby> one.", 4)];

        let hashes = load_dict("en").unwrap();
        let lemma = load_lemma().unwrap();
        let anotator = RubyAnnotator {};
        for (input, output, lvl) in data {
            let result = annotate_phrase(&anotator, input, &hashes, &lemma, 1, false, lvl);
            assert_eq!(result, output);
        }
    }
}