mod annotation;
pub mod cmd;
pub mod html;
pub mod types;
use annotation::{annotate_phrase, load_dict, load_lemma};
use html::process_html;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use tauri::Runtime;
use types::{ProgressReporter, RubyAnnotator};

pub fn process<R: Runtime>(
    file: &str,
    language: &str,
    book_format: &str,
    include_phoneme: bool,
    def_len: i32,
    hint_level: i32,
    reporter: Option<&ProgressReporter<R>>,
) -> Result<(), String> {
    println!("book format: {}", book_format);
    let lemma = load_lemma().unwrap();
    let annotation_dict = load_dict(language).unwrap();
    let ruby_annotator = RubyAnnotator {};

    let process_text_wrapper = Box::new(move |input: &str| {
        if input.trim().is_empty() {
            return input.to_string();
        }

        let res = annotate_phrase(
            &ruby_annotator,
            input,
            &annotation_dict,
            &lemma,
            def_len,
            include_phoneme,
            hint_level,
        );
        res
    });
    let f = Path::new(file);
    let out_file = f.file_stem().unwrap().to_str().unwrap();
    let out_file_ext = f.extension().unwrap().to_str().unwrap();
    let out_file = format!(
        "{}\\{}.out.{}",
        f.parent().unwrap().to_str().unwrap(),
        out_file,
        out_file_ext
    );

    let fn_ptr: &dyn Fn(&str) -> String = process_text_wrapper.as_ref();

    let input = File::open(file).unwrap();
    let mut reader = BufReader::new(input);
    let output = File::create(&out_file).unwrap();
    let mut writer = BufWriter::new(output);
    process_html(&mut reader, &mut writer, fn_ptr, reporter).map_err(|err| err.to_string())?;

    // remove the source file
    std::fs::remove_file(file).map_err(|err| err.to_string())?;
    // replace the source file with new file
    std::fs::rename((&out_file).as_str(), file).map_err(|err| err.to_string())?;

    Ok(())
}
