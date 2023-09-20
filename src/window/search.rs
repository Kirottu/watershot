use std::str::FromStr;

use regex::Regex;
use strum::EnumString;

#[derive(Debug, Clone, Copy, EnumString, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub enum WindowSearchAttribute {
    InitialTitle,
    Title,
    InitialClass,
    Class,
}

#[derive(Debug, Clone)]
pub struct WindowSearchParam {
    pub attribute: WindowSearchAttribute,
    pub value: Regex,
}

impl FromStr for WindowSearchParam {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (attribute, value) = s.split_once('=').ok_or(
            "
        Invalid search parameter. Search parameters should be in the form of
        \"attribute=value\". Valid attributes are \"initial_title\", \"title\",
        \"initial_class\", and \"class\".
        ",
        )?;

        let attribute: WindowSearchAttribute = attribute
            .parse()
            .map_err(|_| format!("Invalid search attribute \"{}\"", attribute))?;

        let value = Regex::new(value).map_err(|_| format!("Invalid search value \"{}\"", value))?;

        Ok(WindowSearchParam { attribute, value })
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_window_search_param_from_str() {
        use WindowSearchAttribute::*;

        let params_and_expected = [
            ("initial_title=^hello$", InitialTitle),
            ("title=^hello$", Title),
            ("initial_class=^hello$", InitialClass),
            ("class=^hello$", Class),
        ];

        for (param_str, expected) in params_and_expected {
            let param: Result<WindowSearchParam, String> = param_str.parse();

            assert!(param.is_ok());

            let param = param.unwrap();

            assert_eq!(param.attribute, expected, "Parsing {param_str}");
            assert_eq!(param.value.as_str(), "^hello$");
        }
    }
}
