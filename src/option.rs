use std::{cell::RefCell, collections::HashMap, fmt::Display, hash::Hash, rc::Rc, str::FromStr, vec};
use std::cell::Ref;
use std::collections::HashSet;
use std::fmt::{Formatter, Pointer};
use std::hash::Hasher;
use std::ops::Deref;

use crate::error::OptionErr;
use crate::util::{OptionValidator, Util};

#[derive(Clone, Debug)]
pub enum ArgCount {
    Fixed(usize),
    Uninitialized,
    Unlimited,
}

impl ArgCount {
    pub fn get_fix_unchecked(&self) -> usize {
        if let Self::Fixed(n) = self {
            return *n;
        } else {
            panic!("Get number on non fixed ArgCount");
        }
    }

    pub fn is_fix(&self) -> bool {
        match self {
            ArgCount::Fixed(_) => true,
            _ => false,
        }
    }

    pub fn is_uninitialized(&self) -> bool {
        match self {
            ArgCount::Uninitialized => true,
            _ => false,
        }
    }

    pub fn is_unlimited(&self) -> bool {
        match self {
            ArgCount::Unlimited => true,
            _ => false,
        }
    }
}

/// The `AnpOption` represents a single option.
///
/// # Examples
///
/// Create an option without argument: `-v,--verbose`
/// ```
/// use anpcli::AnpOption;
/// let opt = AnpOption::builder().option("v").long_option("verbose").build();
/// ```
///
/// Create an option with single argument: `-f <arg>`.
/// ```
/// use anpcli::AnpOption;
/// let opt = AnpOption::builder().option("f").has_arg(true).build();
/// ```
///
/// Create an option with multiple arguments: `--file [<arg> ...]`
/// ```
/// use anpcli::AnpOption;
/// let opt = AnpOption::builder().long_option("file").has_args().optional_arg(true);
/// let another_opt = AnpOption::builder().long_option("file").number_of_args(3);
/// ```
#[derive(Debug)]
pub struct AnpOption {
    option: Option<String>,
    description: Option<String>,
    long_option: Option<String>,
    arg_name: Option<String>,
    required: bool,
    optional_arg: bool,
    arg_count: ArgCount,
    value_sep: Option<char>,
    values: Vec<String>,
}

/// An builder struct for [`AnpOption`].
pub struct OptionBuilder {
    option: Option<String>,
    description: Option<String>,
    long_option: Option<String>,
    arg_name: Option<String>,
    required: bool,
    optional_arg: bool,
    arg_count: ArgCount,
    value_sep: Option<char>,
}

impl OptionBuilder {
    /// Build an [`AnpOption`] with configured values.
    ///
    /// # Error
    ///
    /// Returns an error if:
    /// - `option` and `long_option` are not specified.
    /// - `option` or `long_option` is blank or not valid option name.
    ///
    /// The valid option name:
    /// - for a single char - `alphabetic` only
    /// - for multiple chars - `alphabetic`, `"@"`, `"?"`
    pub fn build(self) -> Result<AnpOption, OptionErr> {
        if self.option.is_none() && self.long_option.is_none() {
            return Err(OptionErr::of(None, "either opt or longOpt must be specified"));
        }
        if let Some(ref option) = self.option {
            OptionValidator::validate(option)?;
        }
        if let Some(ref long_option) = self.long_option {
            if long_option.is_empty() {
                return Err(OptionErr::of(None, "longOpt cannot be blank"));
            }
        }
        Ok(AnpOption {
            option: self.option,
            long_option: self.long_option,
            arg_name: self.arg_name,
            description: self.description,
            required: self.required,
            arg_count: self.arg_count,
            value_sep: self.value_sep,
            optional_arg: self.optional_arg,
            values: Vec::new(),
        })
    }

    /// Set the argument name of the option.
    pub fn arg_name(mut self, arg_name: &str) -> Self {
        self.arg_name = Some(arg_name.to_owned());
        self
    }

    /// Set the short option name.
    ///
    /// The short option name is not necessary to have a single char.
    /// A short option with multiple chars is also valid, like `-required`.
    pub fn option(mut self, opt: &str) -> Self {
        self.option = Some(opt.trim().to_owned());
        self
    }

    /// Set the long option name.
    pub fn long_option(mut self, long_opt: &str) -> Self {
        self.long_option = Some(long_opt.trim().to_owned());
        self
    }

    /// Set the description of the option.
    pub fn desc(mut self, description: &str) -> Self {
        self.description = Some(description.trim().to_owned());
        self
    }

    /// Set whether the option has exactly one argument or no argument.
    ///
    /// Also see [`Self::has_args`] and [`Self::number_of_args`].
    pub fn has_arg(mut self, has_arg: bool) -> Self {
        self.arg_count = if has_arg {
            ArgCount::Fixed(1)
        } else {
            ArgCount::Fixed(0)
        };
        self
    }

    /// Set the option to having unlimited number of arguments.
    ///
    /// Also see [`Self::has_arg`] and [`Self::number_of_args`].
    pub fn has_args(mut self) -> Self {
        self.arg_count = ArgCount::Unlimited;
        self
    }

    /// Set the option to having exactly `number_of_args` number of arguments.
    ///
    /// Also see [`Self::has_arg`] and [`Self::has_args`]
    pub fn number_of_args(mut self, number_of_args: usize) -> Self {
        self.arg_count = ArgCount::Fixed(number_of_args);
        self
    }

    /// Whether argument(s) is optional.
    pub fn optional_arg(mut self, is_optional: bool) -> Self {
        self.optional_arg = is_optional;
        self
    }

    /// Whether the option is required to passed to command line.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Set the value separator for the option.
    ///
    /// For example, when the value separator set to `,`, the option value `-v=1,2,3`
    /// is parsed into three values.
    pub fn value_separator(mut self, value_sep: char) -> Self {
        self.value_sep = Some(value_sep);
        self
    }
}

impl AnpOption {
    /// Create a [`OptionBuilder`] to config the option.
    pub fn builder() -> OptionBuilder {
        OptionBuilder {
            option: None,
            long_option: None,
            arg_name: None,
            description: None,
            required: false,
            arg_count: ArgCount::Uninitialized,
            value_sep: None,
            optional_arg: false,
        }
    }

    /// Check if the option has an argument name.
    pub fn has_arg_name(&self) -> bool {
        self.arg_name.is_some() && !self.arg_name.as_ref().unwrap().is_empty()
    }

    /// Check if the option accepts argument.
    ///
    /// Also see [`Self::has_args`]
    pub fn has_arg(&self) -> bool {
        self.arg_count.is_unlimited()
            || (self.arg_count.is_fix() && self.arg_count.get_fix_unchecked() > 0)
    }

    /// Check if the option accepts more than one arguments.
    ///
    /// Also see [`Self::has_arg`]
    pub fn has_args(&self) -> bool {
        self.arg_count.is_unlimited()
            || (self.arg_count.is_fix() && self.arg_count.get_fix_unchecked() > 1)
    }

    /// Check if the option has a long option name.
    pub fn has_long_opt(&self) -> bool {
        self.long_option.is_some()
    }

    /// Check if the option has a short option name.
    pub fn has_no_value(&self) -> bool {
        self.values.is_empty()
    }

    /// Check whether the argument if optional
    pub fn has_optional_arg(&self) -> bool {
        self.optional_arg
    }

    /// Check whether the option has value separator.
    ///
    /// See [`OptionBuilder::value_separator`]
    pub fn has_value_separator(&self) -> bool {
        self.value_sep.is_some()
    }

    /// Check whether the option is required.
    pub fn is_required(&self) -> bool {
        self.required
    }

    pub fn accepts_arg(&self) -> bool {
        if !(self.has_arg() || self.has_args() || self.has_optional_arg()) {
            return false;
        }
        if self.arg_count.is_uninitialized() {
            return false;
        }
        if self.arg_count.is_fix() && self.values.len() >= self.arg_count.get_fix_unchecked() {
            return false;
        }
        return true;
    }

    pub fn requires_arg(&self) -> bool {
        if self.optional_arg {
            return false;
        }
        if self.arg_count.is_unlimited() {
            return self.values.is_empty();
        }
        return self.accepts_arg();
    }

    fn add(&mut self, value: String) -> Result<(), OptionErr> {
        if !self.accepts_arg() {
            return Err(OptionErr::of(Some(self), "cannot add value, list full"));
        }
        self.values.push(value);
        Ok(())
    }

    pub fn add_value_for_processing(&mut self, value: &str) -> Result<(), OptionErr> {
        if self.arg_count.is_uninitialized() {
            return Err(OptionErr::of(Some(self), "no arg allowed"));
        }
        self.process_value(value)
    }

    fn process_value(&mut self, mut value: &str) -> Result<(), OptionErr> {
        if let Some(value_sep) = self.value_sep {
            let mut index = value.find(value_sep);

            while let Some(i) = index {
                if self.arg_count.is_fix()
                    && self.values.len() == self.arg_count.get_fix_unchecked() - 1
                {
                    break;
                }

                self.add((&value[..i]).to_owned())?;

                value = &value[i + 1..];

                index = value.find(value_sep);
            }
        }

        self.add(value.to_owned())
    }

    pub fn clear_values(&mut self) {
        self.values.clear();
    }

    pub fn get_arg_name(&self) -> Option<&String> {
        self.arg_name.as_ref()
    }

    pub fn get_args(&self) -> &ArgCount {
        &self.arg_count
    }

    pub fn get_description(&self) -> Option<&String> {
        self.description.as_ref()
    }

    pub fn get_key(&self) -> &str {
        if self.option.is_some() {
            self.option.as_ref().unwrap()
        } else {
            self.long_option.as_ref().unwrap()
        }
    }

    pub fn get_id(&self) -> char {
        self.get_key().chars().next().unwrap()
    }

    pub fn get_long_opt(&self) -> Option<&String> {
        self.long_option.as_ref()
    }

    pub fn get_opt(&self) -> Option<&String> {
        self.option.as_ref()
    }

    pub fn get_value<T: FromStr>(&self) -> Option<Result<T, T::Err>> {
        Some(T::from_str(self.values.get(0)?))
    }

    pub fn get_value_at<T: FromStr>(&self, index: usize) -> Option<Result<T, T::Err>> {
        Some(T::from_str(self.values.get(index)?))
    }

    pub fn get_values<T: FromStr>(&self) -> Vec<Result<T, T::Err>> {
        self.values.iter().map(|v| T::from_str(v)).collect()
    }

    pub fn get_value_separator(&self) -> Option<char> {
        self.value_sep
    }

    pub fn set_arg_name(&mut self, arg_name: &str) {
        self.arg_name = Some(arg_name.to_owned());
    }

    pub fn set_args(&mut self, num: usize) {
        self.arg_count = ArgCount::Fixed(num);
    }

    pub fn set_description(&mut self, description: &str) {
        self.description = Some(description.to_owned());
    }

    pub fn set_long_option(&mut self, long_option: &str) {
        self.long_option = Some(long_option.to_owned());
    }

    pub fn set_optional_arg(&mut self, optional_arg: bool) {
        self.optional_arg = optional_arg;
    }

    pub fn set_required(&mut self, required: bool) {
        self.required = required;
    }

    pub fn set_value_separator(&mut self, value_sep: char) {
        self.value_sep = Some(value_sep);
    }
}

impl Clone for AnpOption {
    fn clone(&self) -> Self {
        Self {
            option: self.option.clone(),
            description: self.description.clone(),
            long_option: self.long_option.clone(),
            arg_name: self.arg_name.clone(),
            required: self.required.clone(),
            optional_arg: self.optional_arg.clone(),
            arg_count: self.arg_count.clone(),
            value_sep: self.value_sep.clone(),
            values: Vec::new(),
        }
    }
}

impl Display for AnpOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut buf = String::from("[ option: ");

        buf.push_str(match self.option {
            Some(ref option) => option,
            None => "None",
        });

        if let Some(long_option) = &self.long_option {
            buf.push_str(" ");
            buf.push_str(long_option);
        }

        buf.push_str(" ");

        if self.has_args() {
            buf.push_str("[ARG...]")
        } else if self.has_arg() {
            buf.push_str("[ARG]")
        }

        buf.push_str(" :: ");
        buf.push_str(match self.description {
            Some(ref desc) => desc,
            None => "None",
        });

        buf.push_str(" ]");

        write!(f, "{}", buf)
    }
}

impl PartialEq for AnpOption {
    fn eq(&self, other: &Self) -> bool {
        self.option == other.option && self.long_option == other.long_option
    }
}

impl Hash for AnpOption {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.option.hash(state);
        self.long_option.hash(state);
    }
}

/// The `OptionGroup` is used to specify mutually exclusive options.
///
/// # Example
///
/// Create option group `[-optA | -optB]`
/// ```
/// use anpcli::{AnpOption, OptionGroup, Options};
/// let group = OptionGroup::new()
///     .add_option(AnpOption::builder().option("optA").build().unwrap())
///     .add_option(AnpOption::builder().option("optB").build().unwrap());
///
/// let mut options = Options::new();
/// options.add_option_group(group);
/// ```
#[derive(Debug)]
pub struct OptionGroup {
    option_map: HashMap<String, Rc<RefCell<AnpOption>>>,
    selected: Option<String>,
    required: bool,
}

impl OptionGroup {

    /// Create a new `OptionGroup`.
    pub fn new() -> OptionGroup {
        OptionGroup {
            option_map: HashMap::new(),
            selected: None,
            required: false,
        }
    }

    /// Add an option to the group.
    /// If the same option key already exists, it's a replacement operation.
    pub fn add_option(mut self, option: AnpOption) -> Self {
        self.option_map
            .insert(option.get_key().to_owned(), Rc::new(RefCell::new(option)));
        self
    }

    /// Get the keys of all options in the group.
    /// The key is short option name if exists, otherwise long option name.
    pub fn get_names(&self) -> Vec<&str> {
        self.option_map.keys().map(|k| k.as_str()).collect()
    }

    /// Get the owned reference of the options in the group.
    pub fn get_options(&self) -> Vec<Rc<RefCell<AnpOption>>> {
        self.option_map.values().map(|opt| Rc::clone(opt)).collect()
    }

    /// Get selected option key in the group.
    pub fn get_selected(&self) -> Option<&String> {
        self.selected.as_ref()
    }

    /// Check whether the group is required.
    pub fn is_required(&self) -> bool {
        self.required
    }

    /// Set whether the group is required.
    pub fn set_required(&mut self, required: bool) {
        self.required = required;
    }

    /// Set the selected key in the group.
    /// This is for internal usage.
    pub fn set_selected(&mut self, option: Option<&AnpOption>) -> Result<(), OptionErr> {
        if option.is_none() {
            self.selected = None;
            return Ok(());
        }

        let option = option.unwrap();

        if let Some(selected) = &self.selected {
            if selected != option.get_key() {
                return Err(OptionErr::of(Some(option), "option group already selected"));
            }
        }

        self.selected = Some(option.get_key().to_owned());
        Ok(())
    }
}

impl PartialEq for OptionGroup {
    fn eq(&self, other: &Self) -> bool {
        self.required == other.required
            && self.selected == other.selected
            && self.option_map.keys().collect::<HashSet<&String>>()
            == other.option_map.keys().collect::<HashSet<&String>>()
    }
}

impl Eq for OptionGroup {}

impl Hash for OptionGroup {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u8(u8::from(self.required));

        if let Some(selected) = &self.selected {
            state.write(selected.as_bytes());
        }

        for key in self.option_map.keys() {
            state.write(key.as_bytes());
        }
    }
}

impl Display for OptionGroup {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut buf = String::from("[");

        let options = self.get_options();
        let num = options.len();

        for (i, option) in self.get_options().into_iter().enumerate() {
            if option.borrow().get_opt().is_some() {
                buf.push_str("-");
                buf.push_str(option.borrow().get_opt().unwrap());
            } else {
                buf.push_str("--");
                buf.push_str(option.borrow().get_long_opt().unwrap())
            }

            if let Some(desc) = option.borrow().get_description() {
                buf.push_str(" ");
                buf.push_str(desc);
            }

            if i < num - 1 {
                buf.push_str(", ")
            }
        }

        buf.push_str("]");
        write!(f, "{}", buf)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Required {
    OptKey(String),
    OptGroup(Rc<HashRefCellGroup>),
}

impl Display for Required {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Required::OptKey(key) => Display::fmt(key, f),
            Required::OptGroup(group) => group.fmt(f)
        }
    }
}

/// The `Options` is a collection of [`AnpOption`] and [`OptionGroup`].
///
/// # Examples
///
/// Basic usage
/// ```
/// use anpcli::{AnpOption, Options};
///
/// let mut options = Options::new();
/// options.add_option0("v", false, "print verbosely").unwrap();
/// options.add_option1("output", "output filename").unwrap();
/// options.add_option2("i", "input", true, "input filename").unwrap();
/// options.add_option(AnpOption::builder()
///                     .long_option("algorithm")
///                     .number_of_args(1)
///                     .desc("the algorithm used to process file")
///                     .build().unwrap());
/// ```
///
/// Set default values. Note that if the default values have a key
/// not found in the actual options, the parser will result in [`ParseErr`].
/// ```
/// use std::collections::HashMap;
/// use anpcli::Options;
///
/// let mut defaults = HashMap::new();
/// defaults.insert("target".to_string(), "binary".to_string());
///
/// let mut options = Options::new();
/// options.set_defaults(defaults);
/// options.add_option0("target", true, "the target output format").unwrap();
/// ```
#[derive(Clone)]
pub struct Options {
    short_opts: HashMap<String, Rc<RefCell<AnpOption>>>,
    long_opts: HashMap<String, Rc<RefCell<AnpOption>>>,
    required_opts: Vec<Rc<RefCell<Required>>>,
    option_groups: HashMap<String, Rc<HashRefCellGroup>>,
    defaults: Option<HashMap<String, String>>,
}

impl Options {

    /// Create a new `Options` struct.
    pub fn new() -> Options {
        Options {
            short_opts: HashMap::new(),
            long_opts: HashMap::new(),
            required_opts: Vec::new(),
            option_groups: HashMap::new(),
            defaults: None,
        }
    }

    /// Check if the `Options` has any default value.
    pub fn has_defaults(&self) -> bool {
        self.defaults.is_some()
    }

    /// Set default values for options.
    pub fn set_defaults(&mut self, defaults: HashMap<String, String>) {
        self.defaults = Some(defaults);
    }

    /// Get the immutable reference of the default values if exists.
    pub fn get_defaults(&self) -> Option<&HashMap<String, String>> {
        self.defaults.as_ref()
    }

    /// Add an [`AnpOption`] to the collection.
    ///
    /// Also see [`Self::add_option0`], [`Self::add_option1`], [`Self::add_option2`],
    /// [`Self::add_required_option`]
    pub fn add_option(&mut self, option: AnpOption) {
        let option = Rc::new(RefCell::new(option));
        self.add_option_inner(option);
    }

    fn add_option_inner(&mut self, option: Rc<RefCell<AnpOption>>) {
        if let Some(long_opt) = option.borrow().get_long_opt() {
            self.long_opts
                .insert(long_opt.to_owned(), Rc::clone(&option));
        }

        if option.borrow().is_required() {
            let index = self
                .required_opts
                .iter()
                .position(|v| v.borrow().deref() == &Required::OptKey(option.borrow().get_key().to_owned()));
            if let Some(i) = index {
                self.required_opts.remove(i);
            }
            self.required_opts
                .push(Rc::new(RefCell::new(Required::OptKey(option.borrow().get_key().to_owned()))));
        }

        let key = option.borrow().get_key().to_owned();
        self.short_opts.insert(key, option);
    }

    /// A convenient way to add [`AnpOption`] to the collection.
    ///
    /// Also see [`Self::add_option`], [`Self::add_option1`], [`Self::add_option2`],
    /// [`Self::add_required_option`]
    pub fn add_option0(
        &mut self,
        opt: &str,
        has_arg: bool,
        description: &str,
    ) -> Result<(), OptionErr> {
        let option = AnpOption::builder()
            .option(opt)
            .has_arg(has_arg)
            .desc(description)
            .build()?;
        self.add_option(option);
        Ok(())
    }

    /// A convenient way to add [`AnpOption`] to the collection.
    ///
    /// Also see [`Self::add_option`], [`Self::add_option0`], [`Self::add_option2`],
    /// [`Self::add_required_option`]
    pub fn add_option1(&mut self, opt: &str, description: &str) -> Result<(), OptionErr> {
        self.add_option0(opt, false, description)
    }

    /// A convenient way to add [`AnpOption`] to the collection.
    ///
    /// Also see [`Self::add_option`], [`Self::add_option0`], [`Self::add_option1`],
    /// [`Self::add_required_option`]
    pub fn add_option2(
        &mut self,
        opt: &str,
        long_opt: &str,
        has_arg: bool,
        description: &str,
    ) -> Result<(), OptionErr> {
        let option = AnpOption::builder()
            .option(opt)
            .long_option(long_opt)
            .has_arg(has_arg)
            .desc(description)
            .build()?;
        self.add_option(option);
        Ok(())
    }

    /// Add an option group to the collection.
    pub fn add_option_group(&mut self, group: OptionGroup) {
        let required = group.is_required();
        let group = Rc::new(HashRefCellGroup(RefCell::new(group)));

        if required {
            self.required_opts
                .push(Rc::new(RefCell::new(Required::OptGroup(Rc::clone(&group)))));
        }

        for option in group.borrow().get_options() {
            option.borrow_mut().set_required(false);
            self.add_option_inner(Rc::clone(&option));

            self.option_groups
                .insert(option.borrow().get_key().to_owned(), Rc::clone(&group));
        }
    }

    /// A convenient way to add required [`AnpOption`] to the collection.
    ///
    /// Also see [`Self::add_option`], [`Self::add_option0`], [`Self::add_option1`],
    /// [`Self::add_option2`]
    pub fn add_required_option(
        &mut self,
        opt: &str,
        long_opt: &str,
        has_arg: bool,
        description: &str,
    ) -> Result<(), OptionErr> {
        let opt = AnpOption::builder()
            .option(opt)
            .long_option(long_opt)
            .has_arg(has_arg)
            .required(true)
            .desc(description)
            .build()?;
        self.add_option(opt);
        Ok(())
    }

    /// For internal usage.
    pub fn get_matching_options(&self, opt: &str) -> Vec<String> {
        let opt = Util::strip_leading_hyphens(opt);

        if self.long_opts.contains_key(opt) {
            return vec![opt.to_owned()];
        }

        let mut matching_opts = Vec::new();
        for (key, _) in self.long_opts.iter() {
            if key.starts_with(opt) {
                matching_opts.push(key.to_owned());
            }
        }

        return matching_opts;
    }

    pub fn get_option(&self, opt: &str) -> Option<Rc<RefCell<AnpOption>>> {
        let opt = Util::strip_leading_hyphens(opt);

        if let Some(option) = self.short_opts.get(opt) {
            Some(Rc::clone(option))
        } else if let Some(option) = self.long_opts.get(opt) {
            Some(Rc::clone(option))
        } else {
            None
        }
    }

    pub fn get_option_group(&self, option: &AnpOption) -> Option<Rc<HashRefCellGroup>> {
        if let Some(opt_group) = self.option_groups.get(option.get_key()) {
            Some(Rc::clone(opt_group))
        } else {
            None
        }
    }

    pub fn get_option_groups(&self) -> HashSet<Rc<HashRefCellGroup>> {
        self.option_groups.iter().map(|(_, group)| Rc::clone(group)).collect()
    }

    pub fn get_options(&self) -> Vec<Ref<AnpOption>> {
        self.short_opts.values().map(|x| x.borrow()).collect()
    }

    pub fn get_required_options(&self) -> Vec<Rc<RefCell<Required>>> {
        self.required_opts.iter().map(|r| Rc::clone(r)).collect()
    }

    pub fn has_long_option(&self, opt: &str) -> bool {
        let opt = Util::strip_leading_hyphens(opt);
        self.long_opts.contains_key(opt)
    }

    pub fn has_option(&self, opt: &str) -> bool {
        let opt = Util::strip_leading_hyphens(opt);
        self.short_opts.contains_key(opt) || self.long_opts.contains_key(opt)
    }

    pub fn has_short_option(&self, opt: &str) -> bool {
        let opt = Util::strip_leading_hyphens(opt);
        self.short_opts.contains_key(opt)
    }
}

impl Display for Options {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut buf = String::from("[ Options: [ short {");
        self.short_opts.iter().for_each(|(key, option)| {
            buf.push_str(&format!("{}: {}", key, option.borrow()));
        });
        buf.push_str("[ [ long ");
        self.long_opts.iter().for_each(|(key, option)| {
            buf.push_str(&format!("{}: {}", key, option.borrow()));
        });
        buf.push_str(" ]");

        write!(f, "{}", buf)
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct HashRefCellGroup(RefCell<OptionGroup>);

impl Hash for HashRefCellGroup {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state)
    }
}

impl Deref for HashRefCellGroup {
    type Target = RefCell<OptionGroup>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
