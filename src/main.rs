use std::env;
use std::fs::{File, rename, copy, read_to_string, remove_file};
use std::io::{prelude::*, BufReader, BufWriter};

pub mod utils;
pub use crate::utils::program_options::{ProgramArguments, get_program_args};

#[derive(Debug, Default)]
struct FileIndexPair {
    level: Option<usize>,
    index: Option<usize>,
}

#[derive(Debug, Default)]
struct ItemMergeIterator {
    new_file_pair: FileIndexPair,
    old_file_pair1: FileIndexPair,
    old_file_pair2: FileIndexPair,
}

struct MergeIterator {
    file_level: usize,
    count: usize,
    new_file_count: usize,
    curr_count: usize,
}

impl MergeIterator {
    fn new(file_level: usize, count: usize) -> Self {
        MergeIterator { 
            file_level: file_level, 
            count: count,
            new_file_count: 0,
            curr_count: 0,
        }
    }
}

impl Iterator for MergeIterator {
    type Item = ItemMergeIterator;
    fn next(&mut self) -> Option<ItemMergeIterator> {
        if self.count <= 1 || self.curr_count == self.count {
            return None;
        }
        let mut curr_item: Self::Item = Self::Item::default();
        let new_file_level = self.file_level + 1;
        let new_file_pair: FileIndexPair = FileIndexPair{level: Some(new_file_level), index: Some(self.new_file_count)};
        self.new_file_count += 1;
        let old_file_pair1: FileIndexPair = FileIndexPair{level: Some(self.file_level), index: Some(self.curr_count)};
        self.curr_count += 1;
        let old_file_pair2: FileIndexPair = FileIndexPair{level: Some(self.file_level), index: Some(self.curr_count)};
        self.curr_count += 1;

        curr_item.new_file_pair = new_file_pair;
        curr_item.old_file_pair1 = old_file_pair1;

        if self.curr_count <= self.count {
            curr_item.old_file_pair2 = old_file_pair2;
        }

        if self.curr_count >= self.count {
            self.count = self.new_file_count;
            self.file_level = new_file_level;
            self.new_file_count = 0;
            self.curr_count = 0;
        }

        Some(curr_item)
    }
}

#[derive(Debug)]
struct FileSorter {
    input: String,
    output: String,
    lines_per_file: usize,
}

impl Default for FileSorter {
    fn default() -> Self {
        FileSorter {
            input: String::new(),
            output: String::new(),
            lines_per_file: 10,
        }
    }
}

impl FileSorter {
    fn new(args: &ProgramArguments, lines_per_file: Option<usize>) -> Self {
        FileSorter{ 
            input: args.input.clone(),
            output: args.output.clone(),
            lines_per_file: lines_per_file.unwrap_or(10),
        }
    }

    fn get_sub_file_name(file_level: usize, curr_idx: usize) -> String {
        format!("{}_{}.txt", file_level, curr_idx)
    }

    fn get_sub_file(file_level: usize, curr_idx: usize) -> (File, String) {
        let file_name = FileSorter::get_sub_file_name(file_level, curr_idx);
        match File::create(file_name.clone()) {
            Ok(result) => (result, file_name),
            Err(err) => panic!("Can't create file '{}': {:?}", &file_name, err),
        }
    }

    fn sort_small_file(curr_file_name: &String) {
        let file_data = read_to_string(&curr_file_name).expect("Can't read a small file");
        let mut list: Vec<&str> = file_data.split_ascii_whitespace().collect();
        list.sort_unstable();
        let temp_file_name = "tmp_0_0.txt";
        let mut writer = BufWriter::new(File::create(temp_file_name).expect("Can't create a file to write small data"));
        writer.write(list.join("\n").as_bytes()).expect("Can't write a small file");
        rename(temp_file_name, curr_file_name).expect("Can't rename a small file");
    }

    fn split_files(&self, input_buf: BufReader<File>) -> (usize, usize) {
        let mut curr_lines: usize = 0;
        let mut curr_idx: usize = 0;
        let file_level: usize = 0;
        let (mut curr_file, mut curr_file_name) = FileSorter::get_sub_file(file_level, curr_idx);
        for raw_line in input_buf.lines() {
            let line: String = raw_line.unwrap();
            if let Err(e) = writeln!(curr_file, "{}", line) {
                eprintln!("Couldn't write to file: {}", e);
            }
            curr_lines += 1;
            if curr_lines == self.lines_per_file {
                curr_lines = 0;
                curr_idx += 1;
                drop(curr_file);
                FileSorter::sort_small_file(&curr_file_name);
                let (new_file, new_file_name) = FileSorter::get_sub_file(file_level, curr_idx);
                curr_file = new_file;
                curr_file_name = new_file_name;
            }
        }
        if curr_lines > 0 {
            drop(curr_file);
            FileSorter::sort_small_file(&curr_file_name);
        }

        (file_level, curr_idx + 1)
    }

    fn merge_file(output: &String, file1: FileIndexPair, file2: FileIndexPair) {
        if file1.index == None && file2.index == None {
            panic!("Both files for merge not exists");
        } else if file1.index != None && file2.index == None {
            rename(FileSorter::get_sub_file_name(file1.level.unwrap(), file1.index.unwrap()), output).unwrap();
            return;
        } else if file1.index == None && file2.index != None {
            rename(FileSorter::get_sub_file_name(file2.level.unwrap(), file2.index.unwrap()), output).unwrap();
            return;
        }

        let file_name1 = FileSorter::get_sub_file_name(file1.level.unwrap(), file1.index.unwrap());
        let file_name2 = FileSorter::get_sub_file_name(file2.level.unwrap(), file2.index.unwrap());
        {
            let reader1: BufReader<File> = BufReader::new(File::open(&file_name1).unwrap());
            let reader2: BufReader<File> = BufReader::new(File::open(&file_name2).unwrap());
            let mut last_merged_file: File = match File::create(output.clone()) {
                Ok(result) => result,
                Err(err) => panic!("Can't create file '{}': {:?}", &output, err),
            };
            let mut line1 = String::new();
            let mut line2 = String::new();
            let mut lines1 = reader1.lines();
            let mut lines2 = reader2.lines();

            loop {
                if line1.is_empty() {
                    line1 = match lines1.next() {
                        Some(x) => x.unwrap_or(String::new()),
                        None => String::new(),
                    }
                }
                if line2.is_empty() {
                    line2 = match lines2.next() {
                        Some(x) => x.unwrap_or(String::new()),
                        None => String::new(),
                    }
                }
                if line1.is_empty() && line2.is_empty() {
                    break
                }
                if !line1.is_empty() && (line1 < line2 || line2.is_empty()) {
                    if let Err(e) = writeln!(last_merged_file, "{}", line1) {
                        eprintln!("Couldn't write to file: {}", e);
                    }
                    line1 = String::new();
                } else {
                    if let Err(e) = writeln!(last_merged_file, "{}", line2) {
                        eprintln!("Couldn't write to file: {}", e);
                    }
                    line2 = String::new();
                }
            }
        }
        
        remove_file(file_name1).expect("Can't remove merged file1");
        remove_file(file_name2).expect("Can't remove merged file2");
    }

    pub fn merge_files(file_level: usize, count: usize) -> String {
        let curr_iter: MergeIterator = MergeIterator::new(file_level, count);
        let mut last_merged_filename: String = String::default();
        for item in curr_iter {
            last_merged_filename = FileSorter::get_sub_file_name(item.new_file_pair.level.unwrap(), item.new_file_pair.index.unwrap());
            FileSorter::merge_file(&last_merged_filename, item.old_file_pair1, item.old_file_pair2);
        }

        last_merged_filename
    }

    pub fn sort_file(&self) {
        let input_file: File = File::open(&self.input).unwrap();
        let input_buf: BufReader<File> = BufReader::new(input_file);
        let (file_level, count) = self.split_files(input_buf);
        let result_file: String = FileSorter::merge_files(file_level, count);
        if !result_file.is_empty() {
            rename(&result_file, &self.output).unwrap_or_else(|error| 
                panic!("Can't rename file {} to the output {}: {}", &result_file, &self.output, error)
            );
        } else {
            println!("Input file is small");
            copy(&self.input, &self.output).expect("Can't copy input file");
            FileSorter::sort_small_file(&self.output);
        }
    }
}

fn main() {

    let args: ProgramArguments = get_program_args(&env::args().collect());

    let file_sorter = FileSorter::new(&args, None);
    file_sorter.sort_file();
}
