//! # The AnpCLI Library
//!
//! The AnpCLI library provides an API for parsing command line options.
//! It also able to print help message to show available options.
//!
//! The AnpCLI is simple, easy to learn and use but also highly customizable.
//! It's inspired by the Java library - Apache Commons CLI.
//!
//! AnpCLI supports different types of options:
//!
//! - POSIX like options, for example `tar -zxvf foo.tar.gz`
//! - GNU like long options, for example `du --human-readable --max-depth=1`
//! - Short options with value attached, for example `gcc -O2 foo.c`
//! - Long options with single hyphen, for example `ant -projecthelp`
//!
//! A typical help message displayed by AnpCLI looks like this:
//!
//! ```txt
//! usage: ls
//!  -A,--almost-all          do not list implied . and ..
//!  -a,--all                 do not hide entries starting with .
//!  -B,--ignore-backups      do not list implied entried ending with ~
//!  -b,--escape              print octal escapes for nongraphic characters
//!  --block-size <SIZE>      use SIZE-byte blocks
//!  -c                       with -lt: sort by, and show, ctime (time of last
//!                           modification of file status information) with
//!                           -l:show ctime and sort by name otherwise: sort
//!                           by ctime
//!  -C                       list entries by columns
//! ```
//!
//! # Examples
//!
//! A simple example.
//!
//! ```
//! use anpcli::{AnpOption, Parser, DefaultParser, HelpFormatter, Options};
//!
//! let mut options = Options::new();
//! options.add_option2("A", "almost-all", false, "do not list implied . and ..").unwrap();
//! options.add_option2("a", "all", false, "do not hide entries starting with .").unwrap();
//! options.add_option(AnpOption::builder()
//!                     .long_option("block-size")
//!                     .arg_name("SIZE")
//!                     .has_arg(true)
//!                     .desc("use SIZE-byte blocks")
//!                     .build().unwrap());
//!
//! let mut formatter = HelpFormatter::new("ls");
//! let mut parser = DefaultParser::builder().build();
//! let cmd = parser.parse_or_exit(&options, &formatter);
//!
//! println!("almost-all: {}", cmd.has_option("almost-all"));
//! println!("all: {}", cmd.has_option("all"));
//! if cmd.has_option("block-size") {
//!     println!("block-size: {}", cmd.get_expected_value::<usize>("block-size"));
//! }
//! ```
//!
//! A more complicated example.
//!
//! ```
//! use std::io::stdout;
//! use std::process::exit;
//! use std::time::SystemTime;
//! use anpcli::{AnpOption, Parser, DefaultParser, HelpFormatter, Options};
//!
//! let mut options = Options::new();
//! options.add_option1("d", "show datetime").unwrap();
//! options.add_option(AnpOption::builder()
//!     .long_option("log-level")
//!     .number_of_args(1)
//!     .required(false)
//!     .optional_arg(false)
//!     .desc("The level of log to print in console")
//!     .build().unwrap());
//!
//! let mut formatter = HelpFormatter::new("<file> [<file> ...]");
//! formatter.set_auto_usage(true);
//! formatter.set_header("A file processing tool.");
//!
//! let mut parser = DefaultParser::builder().build();
//! let cmd = parser.parse_args(&options, &vec!["file_tool", "demo.txt", "main.txt"]);
//!
//! if cmd.is_err() {
//!     eprintln!("parse error: {}", cmd.unwrap_err());
//!     exit(1);
//! }
//! let cmd = cmd.unwrap();
//!
//! let files = cmd.get_arg_list();
//! if files.len() <= 1 {
//!     eprintln!("missing option <file>");
//!     formatter.print_help(&mut stdout(), &options);
//!     exit(1);
//! } else {
//!     println!("processing file: {:?}", &files[1..]);
//! }
//! if cmd.has_option("d") {
//!     let datetime = SystemTime::now()
//!                     .duration_since(SystemTime::UNIX_EPOCH)
//!                     .unwrap().as_millis();
//!     println!("datetime={}", datetime);
//! }
//! if cmd.has_option("log-level") {
//!     let log_level = cmd.get_expected_value::<String>("log-level");
//!     println!("log-level={}", log_level);
//! }
//! ```

pub use cmd::CommandLine;
pub use error::ParseErr;
pub use format::HelpFormatter;
pub use option::{AnpOption, OptionBuilder, OptionGroup, Options};
pub use parser::{DefaultParser, Parser, ParserBuilder};

mod format;
mod util;
mod option;
mod cmd;
mod parser;
mod error;
