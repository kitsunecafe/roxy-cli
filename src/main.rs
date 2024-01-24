use clap::Parser as Clap;
use std::{
    fs::File,
    io::{BufReader, Error, ErrorKind, Read},
    path::{Path, PathBuf, StripPrefixError},
};
use toml::Table;

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

impl From<String> for RoxyError {
    fn from(value: String) -> Self {
        Self { message: value }
    }
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

fn get_files<P: AsRef<Path> + std::fmt::Debug>(path: &P) -> Result<Vec<PathBuf>, RoxyError> {
    let path = path
        .as_ref()
        .to_str()
        .ok_or_else(|| RoxyError::from(format!("{path:?} is not a valid path.")))?;

    let files: Vec<PathBuf> = glob(path)?
        .filter_map(|x| x.ok())
        .filter(|f| Path::is_file(f))
        .collect();

    Ok(files)
}

#[derive(Debug)]
struct FilePath<'a, P: AsRef<Path>> {
    input: PathBuf,
    root_dir: PathBuf,
    output: &'a P,
}

impl<'a, P: AsRef<Path> + 'a> FilePath<'a, P> {
    pub fn new(input: &'a P, output: &'a P) -> Self {
        Self {
            input: Self::make_recursive(input),
            root_dir: Self::strip_wildcards(input),
            output,
        }
    }

    fn make_recursive(path: &'a P) -> PathBuf {
        path.as_ref().join("**/*")
    }

    fn has_no_wildcard<S: AsRef<str>>(path: &S) -> bool {
        !path.as_ref().contains("*")
    }

    fn strip_wildcards<P2: AsRef<Path> + ?Sized>(path: &'a P2) -> PathBuf {
        path.as_ref()
            .ancestors()
            .map(Path::to_str)
            .flatten()
            .find(Self::has_no_wildcard)
            .map_or_else(|| PathBuf::new(), PathBuf::from)
    }

    pub fn to_output<P2: AsRef<Path>>(&self, value: &'a P2) -> Result<PathBuf, RoxyError> {
        value
            .as_ref()
            .strip_prefix(&self.root_dir)
            .map(|path| self.output.as_ref().join(path))
            .map_err(RoxyError::from)
    }
}

fn main() -> Result<(), RoxyError> {
    let opts = Options::parse();

    let file_path = FilePath::new(&opts.input, &opts.output);
    let files = get_files(&file_path.input)?;
    let (meta, files): (Vec<&PathBuf>, Vec<&PathBuf>) =
        files.iter().partition(|f| f.extension().unwrap() == "toml");

    let mut context = tera::Context::new();
    for path in meta {
        let mut buf = Vec::new();

        let mut file = File::open(path).map(BufReader::new).unwrap();
        file.read_to_end(&mut buf).unwrap();
        let mut str = String::from_utf8(buf).unwrap();
        let toml: Table = toml::from_str(&mut str).unwrap();

        for (k, v) in toml.iter() {
            context.insert(k, v);
        }
    }

    let mut parser = Parser::new();
    parser.push(Markdown::new());

    let html = Html::new(tera::Tera::default(), context);
    parser.push(html);

    for file in files {
        let file_name = file.with_extension("html");
        let _ = Roxy::process_file(&file, &(&file_path.to_output(&file_name)?), &mut parser);
    }

    Ok(())
}
