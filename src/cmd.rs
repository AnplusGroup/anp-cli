use std::any::type_name;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Deref;
use std::process::exit;
use std::rc::Rc;
use std::str::FromStr;

use crate::option::AnpOption;

/// The `CommandLine` is the struct holding all parsed options and arguments.
///
/// For options, the method `has_option` will return true if that option is specified,
/// regardless of whether it has a value. If the option has values, method `get_value`
/// or `get_values` can be used to retrieve the parsed value of your requested type.
/// The resultant value is `Option<Result<>>` for `get_value` and `Vec<Result<>>`
/// for `get_values`, which is a bit verbose to use. You could use `get_expected_value`
/// and `get_expected_values` instead. They auto unwrap the results and exit with
/// well described error message in case of no value or type conversion error.
///
/// The arguments are the additional values that are not captured by any option.
/// Method `get_arg_list` is to retrieve all arguments in type `Vec<&str>`, the type
/// conversion should be done in user application if needed.
///
#[derive(Debug)]
pub struct CommandLine {
    args: Vec<String>,
    options: Vec<Rc<RefCell<AnpOption>>>,
}

pub struct CmdBuilder {
    command_line: CommandLine,
}

impl CmdBuilder {
    pub fn build(self) -> CommandLine {
        self.command_line
    }

    pub fn add_arg(mut self, arg: &str) -> Self {
        self.command_line.add_arg(arg);
        self
    }

    pub fn add_option(mut self, opt: Rc<RefCell<AnpOption>>) -> Self {
        self.command_line.add_option(opt);
        self
    }
}

impl CommandLine {
    pub fn builder() -> CmdBuilder {
        CmdBuilder {
            command_line: CommandLine { args: vec![], options: vec![] },
        }
    }

    pub fn add_arg(&mut self, arg: &str) {
        self.args.push(arg.to_owned());
    }

    pub fn add_option(&mut self, option: Rc<RefCell<AnpOption>>) {
        self.options.push(option);
    }

    /// Get additional arguments that are not captured by any options.
    ///
    /// The first arguments is typically the filename of the executable.
    pub fn get_arg_list(&self) -> Vec<&str> {
        self.args.iter().map(|a| a.as_str()).collect()
    }

    fn get_option_properties_inner(&self, option: &AnpOption) -> HashMap<String, String> {
        let mut properties = HashMap::new();

        self.options.iter().for_each(|processed_opt| {
            if processed_opt.borrow().deref() == option {
                let values: Vec<String> = processed_opt.borrow().get_values()
                    .into_iter().map(|r| r.unwrap()).collect();

                if values.len() >= 2 {
                    properties.insert(
                        values.get(0).unwrap().to_owned(),
                        values.get(1).unwrap().to_owned());
                } else if values.len() == 1 {
                    properties.insert(values.get(0).unwrap().to_owned(), "true".to_string());
                }
            }
        });

        properties
    }

    /// Get option values as an key-value pair.
    ///
    /// For example, option `--value a b` results in `{"a": "b"}` and option `--value a`
    /// results in `{"a", "true"}`. Note that if the values are more than 2,
    /// remaining values are ignored.
    pub fn get_option_properties(&self, option: &str) -> HashMap<String, String> {
        for processed_option in self.options.iter() {
            let p_opt = &processed_option.borrow();
            if p_opt.get_opt().map(|o| o as &str) == Some(option)
                || p_opt.get_long_opt().map(|o| o as &str) == Some(option) {
                return self.get_option_properties_inner(&p_opt);
            }
        }
        HashMap::new()
    }

    /// Get all [`AnpOption`] that passed to the command line.
    pub fn get_options(&self) -> Vec<Ref<AnpOption>> {
        self.options.iter().map(|o| o.borrow()).collect()
    }

    /// Get parsed option value in requested type.
    ///
    /// [`None`] is returned if no option `opt` or `opt` has no value.
    /// If the `opt` has more than 1 value, the first value is returned.
    ///
    /// The generic type must implement the trait [`FromStr`].
    /// It the generic type is set to [`String`], it's guaranteed that the result is ok.
    ///
    /// Also see [`CommandLine::get_values`].
    pub fn get_value<T: FromStr>(&self, opt: &str) -> Option<Result<T, T::Err>> {
        let option = self.resolve_option(opt)?;
        option.get_value()
    }

    /// Get parsed option values in requested type.
    ///
    /// Empty `Vec` is returned if no option `opt` or `opt` has no value.
    ///
    /// The generic type must implement the trait [`FromStr`].
    /// It the generic type is set to [`String`], it's guaranteed that the result is ok.
    ///
    /// Also see [`CommandLine::get_value`].
    pub fn get_values<T: FromStr>(&self, opt: &str) -> Option<Vec<Result<T, T::Err>>> {
        let option = self.resolve_option(opt)?;
        Some(option.get_values())
    }

    /// Get parsed option value in requested type or exit.
    ///
    /// The method auto unwrap result from [`CommandLine::get_value`].
    /// If the result is [`None`] or [`Err`], the program exit with error message.
    ///
    /// Also see [`CommandLine::get_expected_values`].
    pub fn get_expected_value<T: FromStr + Debug>(&self, opt: &str) -> T {
        if let Some(result) = self.get_value::<String>(opt) {
            self.parse_or_panic(opt, result.unwrap())
        } else {
            eprintln!("error: option '{}' is required", opt);
            exit(1);
        }
    }

    /// Get parsed option values in requested type or exit.
    ///
    /// The method auto unwrap result from [`CommandLine::get_values`].
    /// If any value in `Vec` is [`Err`], the program exit with error message.
    ///
    /// Also see [`CommandLine::get_expected_value`].
    pub fn get_expected_values<T: FromStr + Debug>(&self, opt: &str) -> Vec<T> {
        if let Some(result) = self.get_values::<String>(opt) {
            result.into_iter()
                .map(|v| { self.parse_or_panic(opt, v.unwrap()) })
                .collect()
        } else {
            eprintln!("error: option '{}' is required", opt);
            exit(1);
        }
    }

    fn parse_or_panic<T: FromStr>(&self, opt: &str, value: String) -> T {
        if let Ok(parsed) = T::from_str(&value) {
            return parsed;
        } else {
            eprintln!("parse error: unable to parse option '{}', expect type '{}', got '{}'",
                      opt, type_name::<T>(), value);
            exit(1);
        }
    }

    /// Check if the `opt` is specified in command line.
    pub fn has_option(&self, opt: &str) -> bool {
        self.resolve_option(opt).is_some()
    }

    fn resolve_option(&self, opt: &str) -> Option<Ref<AnpOption>> {
        for option in self.options.iter() {
            if option.borrow().get_opt().map(|s| s.as_str()) == Some(opt)
                || option.borrow().get_long_opt().map(|s| s.as_str()) == Some(opt) {
                return Some(option.borrow());
            }
        }
        None
    }
}
