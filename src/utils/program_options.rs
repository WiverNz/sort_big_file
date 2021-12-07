use args::{Args,ArgsError,};
use getopts::Occur;
use std::process::exit;

extern crate args;
extern crate getopts;

const PROGRAM_DESC: &'static str = "Run this program";
const PROGRAM_NAME: &'static str = "sort_big_file";

pub struct ProgramArguments {
    pub input: String,
    pub output: String,
}

impl Default for ProgramArguments {
    fn default() -> Self {
        ProgramArguments {
            input: String::default(), 
            output: String::default(),
        }
    }
}

pub fn get_program_args(input_args: &Vec<String>) -> ProgramArguments {
    let result_args: ProgramArguments = match parse_args(&input_args) {
        Ok(result) => match result {
            Some(x) => { x },
            None => { println!("empty args");  exit(0) }
        },
        Err(error) => {
            println!("{}", error);
            exit(1);
        }
    };

    result_args
}

fn parse_args(input: &Vec<String>) -> Result<Option<ProgramArguments>, ArgsError> {
    let mut args = Args::new(PROGRAM_NAME, PROGRAM_DESC);
    args.flag("h", "help", "Print the usage menu");
    let input_arg: &str = "input";
    let output_arg: &str = "output";
    args.option("i",
        input_arg,
        "Input file",
        "file_name",
        Occur::Req,
        None);
    args.option("o",
        output_arg,
        "Output file",
        "file_name",
        Occur::Req,
        None);

    args.parse(input)?;

    let help = args.value_of("help")?;
    if help {
        args.full_usage();
        return Ok(None);
    }

    let mut result: ProgramArguments = ProgramArguments::default();
    result.input = args.value_of(input_arg).unwrap_or(result.input);
    result.output = args.value_of(output_arg).unwrap_or(result.output);

    Ok(Some(result))
}