use std::cell::RefCell;
use std::env;
use std::io::stdout;
use std::ops::Deref;
use std::process::exit;
use std::rc::Rc;

use crate::cmd::CommandLine;
use crate::error::ParseErr;
use crate::format::HelpFormatter;
use crate::option::{AnpOption, Options, Required};
use crate::util::Util;

/// The parser trait to parse command line arguments.
pub trait Parser {

    /// Parse arguments from `env::args()` with provided `options`.
    ///
    /// # Error
    ///
    /// If the arguments retrieved from `env::args()` don't meet the requirement of `options`,
    /// [`ParseErr`] is returned.
    ///
    /// Also see [`Self::parse_args`], [`Self::parse_or_exit`]
    fn parse(&mut self, options: &Options) -> Result<CommandLine, ParseErr>;

    /// Parse arguments from `env::args()` with provided `options`.
    ///
    /// # Error
    ///
    /// If the arguments retrieved from `env::args()` don't meet the requirement of `options`,
    /// error message and option help will be print to stderr before exit.
    ///
    /// Also see [`Self::parse_args`], [`Self::parse`]
    fn parse_or_exit(&mut self, options: &Options, formatter: &HelpFormatter) -> CommandLine;

    /// Parse `arguments` with provided `options`.
    ///
    /// # Error
    ///
    /// If the arguments retrieved from `env::args()` don't meet the requirement of `options`,
    /// [`ParseErr`] is returned.
    ///
    /// Also see [`Self::parse_or_exit`], [`Self::parse`]
    fn parse_args<T: ToString>(&mut self, options: &Options, arguments: &[T]) -> Result<CommandLine, ParseErr>;
}

/// The default implementation of [`Parser`] trait.
pub struct DefaultParser {
    cmd: Option<CommandLine>,
    options: Option<Options>,
    stop_at_non_option: bool,
    current_token: Option<String>,
    current_option: Option<Rc<RefCell<AnpOption>>>,
    skip_parsing: bool,
    expected_opts: Option<Vec<Rc<RefCell<Required>>>>,
    allow_partial_matching: bool,
    strip_leading_and_trailing_quotes: Option<bool>,
}

/// A builder struct to create [`DefaultParser`].
pub struct ParserBuilder {
    allow_partial_matching: bool,
    strip_leading_and_trailing_quotes: Option<bool>,
    stop_at_non_option: bool,
}

impl ParserBuilder {
    pub fn build(self) -> DefaultParser {
        DefaultParser {
            cmd: None,
            options: None,
            stop_at_non_option: self.stop_at_non_option,
            current_token: None,
            current_option: None,
            skip_parsing: false,
            expected_opts: None,
            allow_partial_matching: self.allow_partial_matching,
            strip_leading_and_trailing_quotes: self.strip_leading_and_trailing_quotes,
        }
    }

    /// Set whether allow to partially match an option.
    pub fn set_allow_partial_matching(mut self, allow: bool) -> Self {
        self.allow_partial_matching = allow;
        self
    }

    /// Set whether strip leading and trailing quotes in option value.
    pub fn set_strip_leading_and_trailing_quotes(mut self, strip: bool) -> Self {
        self.strip_leading_and_trailing_quotes = Some(strip);
        self
    }

    /// Set whether stop parsing options and consider all remain arguments as arguments.
    ///
    /// If set to `true`, make sure the executable name is not passed to the parser.
    pub fn set_stop_at_non_option(mut self, stop_at_non_option: bool) -> Self {
        self.stop_at_non_option = stop_at_non_option;
        self
    }
}

impl DefaultParser {

    /// Get the builder to config parser.
    pub fn builder() -> ParserBuilder {
        ParserBuilder {
            allow_partial_matching: true,
            strip_leading_and_trailing_quotes: None,
            stop_at_non_option: false,
        }
    }

    fn check_required_args(&self) -> Result<(), ParseErr> {
        if let Some(opt) = &self.current_option {
            if opt.borrow().requires_arg() {
                return Err(ParseErr::MissingArgument(opt.borrow().clone()));
            }
        }
        return Ok(());
    }

    fn check_required_options(&self) -> Result<(), ParseErr> {
        if !self.expected_opts.as_ref().unwrap().is_empty() {
            let opts = self.expected_opts.as_ref().unwrap().iter()
                .map(|r| r.borrow().clone())
                .collect::<Vec<Required>>();
            return Err(ParseErr::MissingOption(opts));
        }
        return Ok(());
    }

    fn get_matching_long_options(&self, token: &str) -> Vec<String> {
        if self.allow_partial_matching {
            return self.options.as_ref().unwrap().get_matching_options(token);
        }
        if self.options.as_ref().unwrap().has_long_option(token) {
            return vec![self.options.as_ref().unwrap().get_option(token).unwrap().borrow()
                .get_long_opt().unwrap().to_owned()];
        }
        return vec![];
    }

    fn handle_concatenated_options(&mut self, token: &str) -> Result<(), ParseErr> {
        for (i, ch) in token.chars().enumerate() {
            if i == 0 {
                continue;
            }

            if let Some(option) = self.options.as_ref().unwrap().get_option(&ch.to_string()) {
                self.handle_option(&option)?;
            } else {
                self.handle_unknown_token(if self.stop_at_non_option && i > 1 { &token[i..] } else { token })?;
                break;
            }

            if let Some(cur_option) = self.current_option.as_ref() {
                if token.chars().count() != (i + 1) {
                    let result = cur_option.borrow_mut().add_value_for_processing(
                        self.strip_leading_and_trailing_quotes_default_off(&token[i + 1..]));
                    if result.is_err() {
                        return Err(ParseErr::ProcessingErr {
                            source: Some(result.unwrap_err()),
                            desc: format!("error occurred when handling concatenated options: {}", token),
                        });
                    }
                    break;
                }
            }
        }
        Ok(())
    }

    fn handle_long_option(&mut self, token: &str) -> Result<(), ParseErr> {
        if token.find('=').is_none() {
            self.handle_long_option_without_equal(token)
        } else {
            self.handle_long_option_with_equal(token)
        }
    }

    fn handle_long_option_with_equal(&mut self, token: &str) -> Result<(), ParseErr> {
        let pos = token.find('=').unwrap();

        let value = &token[pos + 1..];
        let opt = &token[..pos];

        let matching_opts = self.get_matching_long_options(opt);
        if matching_opts.is_empty() {
            self.handle_unknown_token(&self.current_token.as_ref().unwrap().to_owned())
        } else if matching_opts.len() > 1 && !self.options.as_ref().unwrap().has_long_option(opt) {
            Err(ParseErr::AmbiguousOption { input_opt: opt.to_string(), matching_opts })
        } else {
            let key = if self.options.as_ref().unwrap().has_long_option(opt) {
                opt
            } else {
                matching_opts.get(0).unwrap()
            };
            let option = self.options.as_ref().unwrap().get_option(key).unwrap();

            if option.borrow().accepts_arg() {
                self.handle_option(&option)?;
                let result = self.current_option.as_ref().unwrap().borrow_mut().add_value_for_processing(
                    self.strip_leading_and_trailing_quotes_default_off(value)
                );
                if result.is_err() {
                    return Err(ParseErr::ProcessingErr {
                        desc: format!("Error occurred when processing long option with equal: {}", token),
                        source: Some(result.unwrap_err()),
                    });
                }
                self.current_option = None;
            } else {
                self.handle_unknown_token(&self.current_token.as_ref().unwrap().to_owned())?;
            }
            Ok(())
        }
    }

    fn handle_long_option_without_equal(&mut self, token: &str) -> Result<(), ParseErr> {
        let matching_opts = self.get_matching_long_options(token);

        if matching_opts.is_empty() {
            self.handle_unknown_token(&self.current_token.as_ref().unwrap().to_owned())
        } else if matching_opts.len() > 1 && !self.options.as_ref().unwrap().has_long_option(token) {
            Err(ParseErr::AmbiguousOption { matching_opts, input_opt: token.to_string() })
        } else {
            let key = if self.options.as_ref().unwrap().has_long_option(token) {
                token
            } else {
                matching_opts.get(0).unwrap()
            };
            self.handle_option(&self.options.as_ref().unwrap().get_option(key).unwrap())
        }
    }

    fn handle_option(&mut self, option: &Rc<RefCell<AnpOption>>) -> Result<(), ParseErr> {
        self.check_required_args()?;

        let option = Rc::new(RefCell::new(option.borrow().clone()));

        self.update_required_options(option.borrow().deref())?;

        self.cmd.as_mut().unwrap().add_option(Rc::clone(&option));

        if option.borrow().has_arg() {
            self.current_option = Some(option);
        } else {
            self.current_option = None;
        }

        Ok(())
    }

    fn handle_defaults(&mut self) -> Result<(), ParseErr> {
        if !self.options.as_ref().unwrap().has_defaults() {
            return Ok(());
        }
        let defaults = self.options.as_ref().unwrap().get_defaults().unwrap().clone();
        for (option, value) in &defaults {
            if self.options.as_ref().unwrap().get_option(option).is_some() {
                let opt = self.options.as_ref().unwrap().get_option(option).unwrap();
                let group = self.options.as_ref().unwrap().get_option_group(opt.borrow().deref());
                let selected = group.is_some() && group.unwrap().borrow().get_selected().is_some();

                let mut opt_mut = opt.borrow_mut();
                if !self.cmd.as_ref().unwrap().has_option(option) && !selected {
                    if opt_mut.has_arg() {
                        if opt_mut.get_values::<String>().is_empty() {
                            let result = opt_mut.add_value_for_processing(value);
                            if result.is_err() {
                                return Err(ParseErr::ProcessingErr {
                                    source: Some(result.unwrap_err()),
                                    desc: format!("Error occurred when handling default value: {}", option),
                                });
                            }
                        }
                    } else if "yes" != value.to_lowercase() && "true" != value.to_lowercase() && "1" != value {
                        continue;
                    }

                    self.handle_option(&opt)?;
                    self.current_option = None;
                }
            } else {
                return Err(ParseErr::UndefinedDefaultOption { option: option.to_string(), value: value.to_string() });
            }
        }
        Ok(())
    }

    fn handle_short_and_long_option(&mut self, token: &str) -> Result<(), ParseErr> {
        let t = Util::strip_leading_hyphens(token);

        let pos = t.find('=');

        if t.len() == 1 {
            // -s
            if self.options.as_ref().unwrap().has_short_option(t) {
                self.handle_option(self.options.as_ref().unwrap().get_option(t).as_ref().unwrap())?;
            } else {
                self.handle_unknown_token(token)?;
            }
        } else if pos.is_none() {
            // no equal sign found (-xxx)
            if self.options.as_ref().unwrap().has_short_option(t) {
                self.handle_option(self.options.as_ref().unwrap().get_option(t).as_ref().unwrap())?;
            } else if !self.get_matching_long_options(t).is_empty() {
                // -l or -L
                self.handle_long_option_without_equal(token)?;
            } else {
                // -S1S2S3 or -S1S2V
                self.handle_concatenated_options(token)?;
            }
        } else {
            // equal sign found (-xxx=yyy)
            let opt = &t[..pos.unwrap()];
            let value = &t[pos.unwrap() + 1..];

            if opt.len() == 1 {
                // -S=V
                let option = self.options.as_ref().unwrap().get_option(opt);
                if option.as_ref().is_some_and(|o| o.borrow().accepts_arg()) {
                    self.handle_option(option.as_ref().unwrap())?;
                    let result = self.current_option.as_ref().unwrap().borrow_mut().add_value_for_processing(value);
                    if result.is_err() {
                        return Err(ParseErr::ProcessingErr {
                            source: Some(result.unwrap_err()),
                            desc: format!("Error occurred when parsing token: {}", token),
                        });
                    }
                    self.current_option = None;
                } else {
                    self.handle_unknown_token(token)?;
                }
            } else {
                // -L=V or -L=V
                self.handle_long_option_with_equal(token)?;
            }
        }
        Ok(())
    }

    fn handle_token(&mut self, token: String) -> Result<(), ParseErr> {
        self.current_token = Some(token.to_owned());

        if self.skip_parsing {
            self.cmd.as_mut().unwrap().add_arg(&token);
        } else if "--" == token {
            self.skip_parsing = true;
        } else if self.current_option.as_ref().is_some_and(|o| o.borrow().accepts_arg() && self.is_argument(&token)) {
            let result = self.current_option.as_ref().unwrap().borrow_mut().add_value_for_processing(
                self.strip_leading_and_trailing_quotes_default_on(&token));
            if result.is_err() {
                return Err(ParseErr::ProcessingErr {
                    desc: format!("Error occurred when handling token: {}", token),
                    source: Some(result.unwrap_err()),
                });
            }
        } else if token.starts_with("--") {
            self.handle_long_option(&token)?;
        } else if token.starts_with("-") && token != "-" {
            self.handle_short_and_long_option(&token)?;
        } else {
            self.handle_unknown_token(&token)?;
        }

        Ok(())
    }

    fn handle_unknown_token(&mut self, token: &str) -> Result<(), ParseErr> {
        if token.starts_with("-") && token.len() > 1 && !self.stop_at_non_option {
            return Err(ParseErr::UnrecognizedOption(token.to_string()));
        }

        self.cmd.as_mut().unwrap().add_arg(token);
        if self.stop_at_non_option {
            self.skip_parsing = true;
        }
        Ok(())
    }

    fn is_argument(&self, token: &str) -> bool {
        !self.is_option(token) || self.is_negative_number(token)
    }

    fn is_long_option(&self, token: &str) -> bool {
        if !token.starts_with("-") || token.len() == 1 {
            return false;
        }

        let pos = token.find('=');
        let t = if pos.is_none() { token } else { &token[..pos.unwrap()] };

        if !self.get_matching_long_options(t).is_empty() {
            return true;
        }

        return false;
    }

    fn is_negative_number(&self, token: &str) -> bool {
        token.parse::<f64>().is_ok()
    }

    fn is_option(&self, token: &str) -> bool {
        self.is_long_option(token) || self.is_short_option(token)
    }

    fn is_short_option(&self, token: &str) -> bool {
        if !token.starts_with("-") || token.len() == 1 {
            return false;
        }

        let pos = token.find('=');
        let opt_name = if pos.is_none() { &token[1..] } else { &token[1..pos.unwrap()] };
        if self.options.as_ref().unwrap().has_short_option(opt_name) {
            return true;
        }
        if !opt_name.is_empty() && self.options.as_ref().unwrap().has_short_option(&opt_name[..1]) {
            return true;
        }
        return false;
    }

    fn strip_leading_and_trailing_quotes_default_off<'a>(&self, token: &'a str) -> &'a str {
        if self.strip_leading_and_trailing_quotes.unwrap_or(false) {
            Util::strip_leading_and_trailing_quotes(token)
        } else {
            token
        }
    }

    fn strip_leading_and_trailing_quotes_default_on<'a>(&self, token: &'a str) -> &'a str {
        if self.strip_leading_and_trailing_quotes.unwrap_or(true) {
            Util::strip_leading_and_trailing_quotes(token)
        } else {
            token
        }
    }

    fn update_required_options(&mut self, option: &AnpOption) -> Result<(), ParseErr> {
        if option.is_required() {
            let pos = self.expected_opts.as_ref().unwrap().iter()
                .position(|r| r.borrow().deref() == &Required::OptKey(option.get_key().to_owned()));
            if pos.is_some() {
                self.expected_opts.as_mut().unwrap().remove(pos.unwrap());
            }
        }

        if let Some(group) = self.options.as_ref().unwrap().get_option_group(option) {
            if group.borrow().is_required() {
                let pos = self.expected_opts.as_ref().unwrap().iter()
                    .position(|r| r.borrow().deref() == &Required::OptGroup(Rc::clone(&group)));
                if pos.is_some() {
                    self.expected_opts.as_mut().unwrap().remove(pos.unwrap());
                }
            }

            let result = group.borrow_mut().set_selected(Some(option));
            if result.is_err() {
                return Err(ParseErr::ProcessingErr {
                    source: Some(result.unwrap_err()),
                    desc: format!("error occurred when updating required options"),
                });
            }
        }

        Ok(())
    }
}

impl Parser for DefaultParser {
    fn parse(&mut self, options: &Options) -> Result<CommandLine, ParseErr> {
        self.parse_args(options, &env::args().collect::<Vec<String>>())
    }

    fn parse_or_exit(&mut self, options: &Options, formatter: &HelpFormatter) -> CommandLine {
        let result = self.parse(options);
        if let Ok(cmd) = result {
            return cmd;
        } else {
            let mut error = String::new();
            formatter.render_wrapped_text_block(&mut error, 0, &format!("{}", result.err().unwrap()));
            eprintln!("{}", error);
            println!("{}", "-".repeat(formatter.get_width()));
            formatter.print_help(&mut stdout(), &options);
            exit(1);
        }
    }

    fn parse_args<T>(&mut self, options: &Options, arguments: &[T]) -> Result<CommandLine, ParseErr>
        where T: ToString {
        self.options = Some(options.clone());
        for group in self.options.as_mut().unwrap().get_option_groups() {
            group.borrow_mut().set_selected(None).expect("should succeed");
        }

        self.skip_parsing = false;
        self.current_option = None;
        self.expected_opts = Some(Vec::from(self.options.as_ref().unwrap().get_required_options()));

        self.cmd = Some(CommandLine::builder().build());

        for argument in arguments {
            self.handle_token(argument.to_string())?;
        }

        self.check_required_args()?;

        self.handle_defaults()?;

        self.check_required_options()?;

        Ok(self.cmd.take().unwrap())
    }
}
