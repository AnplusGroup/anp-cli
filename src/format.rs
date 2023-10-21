use std::cmp::Ordering;
use std::io::{BufRead, Cursor, Write};
use std::ops::Deref;
use std::rc::Rc;

use crate::option::{AnpOption, OptionGroup, Options};

pub const DEFAULT_LINE_SEPARATOR: &str = if cfg!(windows) { "\r\n" } else { "\n" };

const DEFAULT_WIDTH: usize = 74;
const DEFAULT_LEFT_PAD: usize = 4;
const DEFAULT_DESC_PAD: usize = 4;
const DEFAULT_SYNTAX_PREFIX: &str = "usage: ";
const DEFAULT_OPT_PREFIX: &str = "-";
const DEFAULT_LONG_OPT_PREFIX: &str = "--";
const DEFAULT_ARG_NAME: &str = "arg";

/// `HelpFormatter` helps print usage information for the [`Options`].
///
/// The output format is like:
/// ```txt
/// usage: <cmd_syntax> [opt_usage]
/// [header]
///     -<opt>, --<long_opt>               <description>
///     -<opt>                             <description>
///     --<long_opt>                       <description>
///     -<opt>, --<long_opt>=[opt_name]    <description>
/// [footer]
/// ```
pub struct HelpFormatter {
    width: usize,
    left_pad: usize,
    desc_pad: usize,
    syntax_prefix: String,
    newline: String,
    opt_prefix: String,
    long_opt_prefix: String,
    arg_name: String,
    option_comparator: Option<Box<dyn Fn(&AnpOption, &AnpOption) -> Ordering>>,
    cmd_syntax: String,
    auto_usage: bool,
    header: Option<String>,
    footer: Option<String>,
}

impl HelpFormatter {
    /// Create a `HelpFormatter` with default configuration.
    ///
    /// The `cmd_syntax` is typically the name of the executable with positional options.
    /// For example, `"cd [<path>]"`.
    pub fn new(cmd_syntax: &str) -> HelpFormatter {
        HelpFormatter {
            width: DEFAULT_WIDTH,
            left_pad: DEFAULT_LEFT_PAD,
            desc_pad: DEFAULT_DESC_PAD,
            syntax_prefix: DEFAULT_SYNTAX_PREFIX.to_string(),
            newline: DEFAULT_LINE_SEPARATOR.to_string(),
            opt_prefix: DEFAULT_OPT_PREFIX.to_string(),
            long_opt_prefix: DEFAULT_LONG_OPT_PREFIX.to_string(),
            arg_name: DEFAULT_ARG_NAME.to_string(),
            option_comparator: Some(Box::new(|x, y| x.get_key().cmp(y.get_key()))),
            cmd_syntax: cmd_syntax.to_string(),
            auto_usage: false,
            header: None,
            footer: None,
        }
    }

    fn append_option(&self, buff: &mut String, option: &AnpOption, required: bool) {
        if !required {
            buff.push_str("[");
        }

        if let Some(opt) = option.get_opt() {
            buff.push_str("-");
            buff.push_str(opt);
        } else {
            buff.push_str("--");
            buff.push_str(option.get_long_opt().unwrap());
        }

        if option.has_arg() && (option.get_arg_name().is_none() || !option.get_arg_name().unwrap().is_empty()) {
            buff.push_str(" ");
            buff.push_str("<");
            buff.push_str(if option.get_arg_name().is_some() { option.get_arg_name().unwrap() } else { self.get_arg_name() });
            buff.push_str(">");
        }

        if !required {
            buff.push_str("]");
        }
    }

    fn append_option_group(&self, buff: &mut String, group: &OptionGroup) {
        if !group.is_required() {
            buff.push_str("[")
        }

        let mut options = group.get_options();
        if let Some(comparator) = self.get_option_comparator() {
            options.sort_by(|a, b| comparator(a.borrow().deref(), b.borrow().deref()));
        }

        let len = options.len();
        for (i, opt) in options.into_iter().enumerate() {
            self.append_option(buff, &opt.borrow(), true);

            if i != (len - 1) {
                buff.push_str(" | ");
            }
        }

        if !group.is_required() {
            buff.push_str("]")
        }
    }

    /// Retrieve the option comparator, which is used to sort the [`AnpOption`]
    /// when printing options.
    pub fn get_option_comparator(&self) -> Option<&dyn Fn(&AnpOption, &AnpOption) -> Ordering> {
        self.option_comparator.as_ref().map(|c| c.deref())
    }

    fn create_padding(&self, len: usize) -> String {
        " ".repeat(len)
    }

    fn find_wrap_pos(&self, text: &str, width: usize, start_pos: usize) -> Option<usize> {
        let trunc_text = &text[start_pos..];

        let pos = trunc_text.find('\n');
        if pos.is_some() && pos.as_ref().unwrap() <= &width {
            return Some(pos.unwrap() + start_pos + 1);
        }

        let pos = trunc_text.find('\t');
        if pos.is_some() && pos.as_ref().unwrap() <= &width {
            return Some(pos.unwrap() + start_pos + 1);
        }

        if start_pos + width >= text.len() {
            return None;
        }

        let mut pos = None;
        for i in (start_pos..start_pos + width + 1).rev() {
            let c = *text.as_bytes().get(i).unwrap() as char;
            if c == ' ' || c == '\r' || c == '\n' {
                pos = Some(i);
                break;
            }
        }

        if pos.is_some() && pos.as_ref().unwrap() > &start_pos {
            return pos;
        }

        let mut len = width;
        while len > 0 {
            if text.is_char_boundary(start_pos + len) {
                return Some(start_pos + width);
            }
            len -= 1;
        }
        panic!("should not happen");
    }

    /// Get the argument name displayed in usage.
    pub fn get_arg_name(&self) -> &str {
        &self.arg_name
    }

    /// Get number of padding space for option description.
    pub fn get_desc_padding(&self) -> usize {
        self.desc_pad
    }

    /// Get number of padding space before option.
    pub fn get_left_padding(&self) -> usize {
        self.left_pad
    }

    /// Get the long option prefix.
    pub fn get_long_opt_prefix(&self) -> &str {
        &self.long_opt_prefix
    }

    /// Get the newline.
    /// For windows, it defaults to `\r\n`.
    /// For other operating system, it defaults to `\n`.
    pub fn get_newline(&self) -> &str {
        &self.newline
    }

    /// Get the option prefix.
    pub fn get_opt_prefix(&self) -> &str {
        &self.opt_prefix
    }

    /// Get the syntax prefix.
    pub fn get_syntax_prefix(&self) -> &str {
        &self.syntax_prefix
    }

    /// Get the max width of the output message.
    pub fn get_width(&self) -> usize {
        self.width
    }

    /// Set the argument name displayed in option usage.
    pub fn set_arg_name(&mut self, arg_name: &str) {
        self.arg_name = arg_name.to_string();
    }

    /// Set number of padding space for option description.
    pub fn set_desc_padding(&mut self, padding: usize) {
        self.desc_pad = padding;
    }

    /// Set number of padding space before option.
    pub fn set_left_padding(&mut self, padding: usize) {
        self.left_pad = padding;
    }

    /// Set the newline characters.
    pub fn set_newline(&mut self, newline: &str) {
        self.newline = newline.to_string();
    }

    /// Set the option comparator, which is used to sort the [`AnpOption`]
    /// when printing options.
    pub fn set_opt_comparator(&mut self, comparator: Option<Box<dyn Fn(&AnpOption, &AnpOption) -> Ordering>>) {
        self.option_comparator = comparator;
    }

    /// Set the syntax prefix, the default value is [`DEFAULT_SYNTAX_PREFIX`].
    pub fn set_syntax_prefix(&mut self, prefix: &str) {
        self.syntax_prefix = prefix.to_string();
    }

    /// Set the maximum width of the display message, which defaults to [`DEFAULT_WIDTH`].
    pub fn set_width(&mut self, width: usize) {
        self.width = width.max(2);
    }

    /// Set the cmd syntax, for display purpose only.
    ///
    /// The `cmd_syntax` is typically the name of the executable with positional options.
    /// For example, `"cd [<path>]"`.
    pub fn set_cmd_syntax(&mut self, syntax: &str) {
        self.cmd_syntax = syntax.to_string();
    }

    /// Set header message.
    pub fn set_header(&mut self, header: &str) {
        self.header = Some(header.to_string());
    }

    /// Set footer message.
    pub fn set_footer(&mut self, footer: &str) {
        self.footer = Some(footer.to_string());
    }

    /// Set if auto print the option usage after `cmd_syntax`.
    pub fn set_auto_usage(&mut self, auto_usage: bool) {
        self.auto_usage = auto_usage;
    }

    /// Print help message of the [`Options`] to the `out` sinks.
    ///
    /// # Example
    ///
    /// ```
    /// use std::io::{stderr};
    /// use anpcli::{HelpFormatter, Options};
    /// HelpFormatter::new("ls").print_help(&mut stderr(), &Options::new());
    /// ```
    pub fn print_help<T: Write>(&self, out: &mut T, options: &Options) {
        if self.auto_usage {
            self.print_usage_with_options(out, options);
        } else {
            self.print_usage(out);
        }

        write!(out, "{}", self.get_newline()).unwrap();

        if self.header.as_ref().is_some_and(|h| !h.is_empty()) {
            self.print_wrapped(out, self.header.as_ref().unwrap());
            write!(out, "{}", self.get_newline()).unwrap();
        }

        self.print_options(out, options);

        if self.footer.as_ref().is_some_and(|f| !f.is_empty()) {
            write!(out, "{}", self.get_newline()).unwrap();
            self.print_wrapped(out, self.footer.as_ref().unwrap());
        }

        write!(out, "{}", self.get_newline()).unwrap();
    }

    /// Print detailed information for options only.
    ///
    /// Also see [`HelpFormatter`],  [`HelpFormatter::print_help`].
    pub fn print_options<T: Write>(&self, out: &mut T, options: &Options) {
        let mut buff = String::new();
        self.render_options(&mut buff, options);
        write!(out, "{}", buff).unwrap();
    }

    /// Print cmd syntax without option usage.
    ///
    /// Also see [`HelpFormatter`],  [`HelpFormatter::print_help`].
    pub fn print_usage<T: Write>(&self, out: &mut T) {
        let arg_pos = self.cmd_syntax.find(' ').map(|x| x + 1).unwrap_or(0);

        self.print_wrapped_with_tab(
            out, &format!("{}{}", self.get_syntax_prefix(), self.cmd_syntax),
            self.get_syntax_prefix().len() + arg_pos);
    }

    /// Print cmd syntax with option usage.
    ///
    /// Also see [`HelpFormatter`],  [`HelpFormatter::print_help`].
    pub fn print_usage_with_options<T: Write>(&self, out: &mut T, options: &Options) {
        let mut buff = format!("{}{} ", self.get_syntax_prefix(), self.cmd_syntax);

        let mut processed_groups = vec![];

        let mut opt_list = options.get_options();
        if self.get_option_comparator().is_some() {
            let cmp = self.get_option_comparator().unwrap();
            opt_list.sort_by(|x, y| cmp(&x, &y));
        }

        let len = opt_list.len();
        for (i, opt) in opt_list.into_iter().enumerate() {
            let group = options.get_option_group(&opt);
            if let Some(group) = group {
                if !processed_groups.contains(&group) {
                    processed_groups.push(Rc::clone(&group));

                    self.append_option_group(&mut buff, &group.borrow())
                }
            } else {
                self.append_option(&mut buff, &opt, opt.is_required());
            }

            if i != len - 1 {
                buff.push_str(" ");
            }
        }

        let tab = buff.find(' ').map(|x| x + 1).unwrap_or(0);
        self.print_wrapped_with_tab(out, &buff, tab);
    }

    fn print_wrapped<T: Write>(&self, out: &mut T, text: &str) {
        self.print_wrapped_with_tab(out, text, 0);
    }

    fn print_wrapped_with_tab<T: Write>(&self, out: &mut T, text: &str, next_line_tap_stop: usize) {
        let mut buff = String::new();
        self.render_wrapped_text_block(&mut buff, next_line_tap_stop, text);
        write!(out, "{}", buff).unwrap();
    }

    fn render_options(&self, buff: &mut String, options: &Options) {
        let left_pad = self.create_padding(self.get_left_padding());
        let desc_pad = self.create_padding(self.get_desc_padding());

        let mut max = 0;
        let mut prefix_list: Vec<String> = vec![];
        let mut opt_list = options.get_options();

        if let Some(cmp) = self.get_option_comparator() {
            opt_list.sort_by(|x, y| cmp(&x, &y));
        }

        for option in opt_list.iter() {
            let mut opt_buff = String::new();

            opt_buff.push_str(&left_pad);
            if option.get_opt().is_none() {
                opt_buff.push_str(self.get_long_opt_prefix());
                opt_buff.push_str(option.get_long_opt().unwrap());
            } else {
                opt_buff.push_str(self.get_opt_prefix());
                opt_buff.push_str(option.get_opt().unwrap());

                if option.has_long_opt() {
                    opt_buff.push_str(", ");
                    opt_buff.push_str(self.get_long_opt_prefix());
                    opt_buff.push_str(option.get_long_opt().unwrap());
                }
            }

            if option.has_arg() {
                let arg_name = option.get_arg_name();
                if arg_name.is_some() && arg_name.as_ref().unwrap().is_empty() {
                    opt_buff.push_str(" ");
                } else {
                    opt_buff.push_str(" ");
                    let arg = if arg_name.is_some() { arg_name.unwrap() } else { self.get_arg_name() };
                    opt_buff.push_str(&format!("<{}>", arg));
                }
            }
            max = max.max(opt_buff.len());
            prefix_list.push(opt_buff);
        }

        let len = opt_list.len();
        for (i, option) in opt_list.into_iter().enumerate() {
            let mut opt_buff = String::from(prefix_list.get(i).unwrap());

            if opt_buff.len() < max {
                opt_buff.push_str(&self.create_padding(max - opt_buff.len()));
            }

            opt_buff.push_str(&desc_pad);

            let next_line_tab_stop = max + self.get_desc_padding();

            if let Some(desc) = option.get_description() {
                opt_buff.push_str(desc);
            }

            self.render_wrapped_text(buff, next_line_tab_stop, &opt_buff);

            if i != len - 1 {
                buff.push_str(self.get_newline());
            }
        }
    }

    fn render_wrapped_text(&self, buff: &mut String, mut next_line_tab_stop: usize, text: &str) {
        let mut pos = self.find_wrap_pos(text, self.get_width(), 0);

        if pos.is_none() {
            buff.push_str(text.trim_end());
            return;
        }
        buff.push_str(&text[..pos.unwrap()].trim_end());
        buff.push_str(self.get_newline());

        if next_line_tab_stop >= self.get_width() || next_line_tab_stop == 0 {
            next_line_tab_stop = 1;
        }

        let mut processing_text = text.to_string();
        let padding = self.create_padding(next_line_tab_stop);
        loop {
            processing_text = format!("{}{}", &padding, &processing_text[pos.unwrap()..].trim());
            pos = self.find_wrap_pos(&processing_text, self.get_width(), 0);

            if pos.is_none() {
                buff.push_str(&processing_text);
                return;
            }

            if processing_text.len() > self.get_width() && pos == Some(next_line_tab_stop - 1) {
                pos = Some(self.get_width());
            }

            buff.push_str(&processing_text[..pos.unwrap()].trim_end());
            buff.push_str(self.get_newline());
        }
    }

    /// Render a wrapped text block to the `buffer` with the max `width` configured.
    /// When text is wrapped, `next_line_tab_stop` number of space is appended.
    pub fn render_wrapped_text_block(&self, buffer: &mut String, next_line_tab_stop: usize, text: &str) {
        let cursor = Cursor::new(text);
        for (i, line) in cursor.lines().map(|l| l.unwrap()).enumerate() {
            if i != 0 {
                buffer.push_str(self.get_newline());
            }
            self.render_wrapped_text(buffer, next_line_tab_stop, &line);
        }
    }
}
