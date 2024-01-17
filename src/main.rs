use clap::Parser as Clap;
use std::{
    ffi,
    io::{Error, ErrorKind},
    ops::Deref,
    path::{Path, PathBuf, StripPrefixError},
};

use glob::{glob, PatternError};
use roxy_core::roxy::{Html, Markdown, Parser, Roxy};

#[derive(Clap)]
#[command(name = "Roxy")]
#[command(author = "KitsuneCafe")]
#[command(version = "2.0")]
#[command(about = "A very small static site generator", long_about = None)]
pub struct Options {
    pub input: String,
    pub output: String,
}

#[derive(Debug)]
struct RoxyError {
    message: String,
}

impl From<PatternError> for RoxyError {
    fn from(value: PatternError) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

impl From<StripPrefixError> for RoxyError {
    fn from(value: StripPrefixError) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

impl From<RoxyError> for Error {
    fn from(value: RoxyError) -> Self {
        Error::new(ErrorKind::Other, value.message)
    }
}

fn get_files(path: &str) -> Result<Vec<PathBuf>, RoxyError> {
    glob(path)
        .map(|p| p.filter_map(|x| x.ok()).collect())
        .map_err(RoxyError::from)
}

struct FilePath<'a, P: AsRef<Path>> {
    input: &'a P,
    output: &'a P,
}

impl<'a, P: AsRef<Path> + 'a> FilePath<'a, P> {
    pub fn new(input: &'a P, output: &'a P) -> Self {
        Self { input, output }
    }

    fn has_wildcard(path: &str) -> bool {
        path.contains("*")
    }

    fn strip_wildcards<P2: AsRef<Path> + ?Sized>(path: &'a P2) -> PathBuf {
        path.as_ref()
            .ancestors()
            .map(Path::to_str)
            .flatten()
            .inspect(|f| println!("{f}"))
            .skip_while(Self::has_wildcard)
            .collect()
    }

    pub fn to_output<P2: AsRef<Path>>(&self, value: &'a P2) -> Result<PathBuf, RoxyError> {
        value
            .as_ref()
            .strip_prefix(Self::strip_wildcards(self.input))
            .map(|path| self.output.as_ref().join(path))
            .map_err(RoxyError::from)
    }
}

fn main() -> Result<(), RoxyError> {
    let opts = Options::parse();
    let mut parser = Parser::new();
    parser.push(Markdown::new());
    let html = Html::default();
    parser.push(html);
    let file_path = FilePath::new(&opts.input, &opts.output);

    for file in get_files(&opts.input)? {
        Roxy::process_file(&file, &(&file_path.to_output(&file)?), &mut parser);
    }

    Ok(())
}
