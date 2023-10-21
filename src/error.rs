use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

use crate::option::{AnpOption, Required};

/// Argument parsing error.
#[derive(Debug)]
pub enum ParseErr {
    /// Missing required [`AnpOption`] or [`OptionGroup`].
    MissingOption(Vec<Required>),

    /// Missing argument(s) passed to [`AnpOption`].
    MissingArgument(AnpOption),

    /// Unknown error when processing options, possibly a bug.
    ProcessingErr {
        desc: String,
        source: Option<OptionErr>,
    },

    /// When `allow_partial_matching` is enabled in [`DefaultParser`] and
    /// multiple [`AnpOption`]s are matched, the error is raised.
    AmbiguousOption {
        input_opt: String,
        matching_opts: Vec<String>,
    },

    /// Unrecognized option is passed to command line.
    UnrecognizedOption(String),

    /// The specified default values have a key that matches no [`AnpOption`].
    UndefinedDefaultOption {
        option: String,
        value: String,
    },
}

impl ParseErr {}

impl Display for ParseErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut msg = String::new();
        match self {
            ParseErr::MissingOption(opt_list) => {
                msg.push_str("missing option '");
                msg.push_str(&match opt_list.first().unwrap() {
                    Required::OptKey(key) => key.to_string(),
                    Required::OptGroup(group) => {
                        group.borrow().get_options().iter()
                            .map(|opt| opt.borrow())
                            .fold(String::new(), |mut a, b| {
                                if !a.is_empty() {
                                    a.push_str(" | ");
                                }
                                a.push_str(b.get_key());
                                return a;
                            })
                    }
                });
                msg.push_str("'");
            }
            ParseErr::MissingArgument(option) => {
                msg.push_str("missing argument for option '");
                msg.push_str(option.get_key());
                msg.push_str("'");
            }
            ParseErr::ProcessingErr { desc, source } => {
                if let Some(err) = source {
                    msg.push_str(&format!("{}", err));
                } else {
                    msg.push_str(desc);
                }
            }
            ParseErr::AmbiguousOption { input_opt, matching_opts } => {
                msg.push_str("ambiguous option '");
                msg.push_str(input_opt);
                msg.push_str("', possible options are ");
                msg.push_str(&matching_opts.join(", "));
            }
            ParseErr::UnrecognizedOption(opt) => {
                msg.push_str("unrecognized option '");
                msg.push_str(opt);
                msg.push_str("'");
            }
            ParseErr::UndefinedDefaultOption { option, .. } => {
                msg.push_str("undefined default option '");
                msg.push_str(option);
                msg.push_str("'");
            }
        };
        write!(f, "parse error, {}", &msg)
    }
}

impl Error for ParseErr {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let Self::ProcessingErr { source, .. } = &self {
            return source.as_ref().map(|s| s as &dyn Error);
        }
        return None;
    }
}

#[derive(Debug)]
pub struct OptionErr {
    option: Option<AnpOption>,
    description: String,
}

impl OptionErr {
    pub fn of(option: Option<&AnpOption>, desc: &str) -> OptionErr {
        OptionErr {
            option: option.map(|o| o.clone()),
            description: desc.to_string(),
        }
    }
}

impl Display for OptionErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut opt_msg = String::new();
        if let Some(opt) = self.option.as_ref() {
            opt_msg.push_str("for option '");
            opt_msg.push_str(opt.get_key());
            opt_msg.push_str("', ");
        }
        write!(f, "{opt_msg}{}", &self.description)
    }
}

impl Error for OptionErr {}
