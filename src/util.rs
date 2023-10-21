use crate::error::OptionErr;

pub struct Util;

impl Util {
    pub fn strip_leading_and_trailing_quotes(string: &str) -> &str {
        let len = string.len();
        if len > 1 && string.starts_with('"') && string.ends_with('"') {
            if !(&string[1..len - 1]).contains('"') {
                return &string[1..len - 1];
            }
        }
        return string;
    }

    pub fn strip_leading_hyphens(string: &str) -> &str {
        if string.starts_with("--") {
            &string[2..]
        } else if string.starts_with("-") {
            &string[1..]
        } else {
            string
        }
    }
}

pub struct OptionValidator;

impl OptionValidator {
    fn is_valid_char(c: char) -> bool {
        char::is_ascii_alphabetic(&c)
    }

    fn is_valid_opt(c: char) -> bool {
        Self::is_valid_char(c) || c == '?' || c == '@'
    }

    pub fn validate(option: &str) -> Result<(), OptionErr> {
        if option.is_empty() {
            return Err(OptionErr::of(None, "illegal blank option name"));
        } else if option.len() == 1 {
            let c = option.chars().into_iter().next().unwrap();

            if !Self::is_valid_opt(c) {
                return Err(OptionErr::of(None, &format!("illegal option name '{}'", c)));
            }
        } else {
            for c in option.chars() {
                if !Self::is_valid_opt(c) {
                    return Err(OptionErr::of(None,
                                             &format!("the option '{}' contains an illegal character: '{}'", option, c)));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::util::{OptionValidator, Util};

    #[test]
    fn test_strip_leading_and_trailing_quotes() {
        assert_eq!("text", Util::strip_leading_and_trailing_quotes("\"text\""));
        assert_eq!("\"text", Util::strip_leading_and_trailing_quotes("\"text"));
        assert_eq!("text\"", Util::strip_leading_and_trailing_quotes("text\""));
        assert_eq!("\"te\"xt\"", Util::strip_leading_and_trailing_quotes("\"te\"xt\""));
        assert_eq!("\"", Util::strip_leading_and_trailing_quotes("\""));
    }

    #[test]
    fn test_strip_leading_hyphens() {
        assert_eq!("option", Util::strip_leading_hyphens("--option"));
        assert_eq!("option", Util::strip_leading_hyphens("-option"));
        assert_eq!("-option", Util::strip_leading_hyphens("---option"));
        assert_eq!("option", Util::strip_leading_hyphens("option"));
        assert_eq!("", Util::strip_leading_hyphens(""));
    }

    #[test]
    fn test_option_validator() {
        assert!(OptionValidator::validate("").is_err());
        assert!(OptionValidator::validate("abc").is_ok());
        assert!(OptionValidator::validate("?").is_ok());
        assert!(OptionValidator::validate("@").is_ok());
        assert!(OptionValidator::validate("5").is_err());
        assert!(OptionValidator::validate("a").is_ok());
        assert!(OptionValidator::validate("z").is_ok());
        assert!(OptionValidator::validate("A").is_ok());
        assert!(OptionValidator::validate("Z").is_ok());
        assert!(OptionValidator::validate("--err").is_err());
        assert!(OptionValidator::validate("ok").is_ok());
        assert!(OptionValidator::validate("@ok").is_ok());
        assert!(OptionValidator::validate("o8k").is_err());
    }
}
