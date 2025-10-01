use std::borrow::Cow;

const WINDOWS_RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com0", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
    "com8", "com9", "lpt0", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

fn is_illegal_char(ch: char) -> bool {
    matches!(ch, '/' | '?' | '<' | '>' | '\\' | ':' | '*' | '|' | '"')
}

fn is_control_char(ch: char) -> bool {
    let code = ch as u32;
    (code <= 0x1f) || (code >= 0x80 && code <= 0x9f)
}

fn replace_illegal_or_control_char<'a>(
    name: impl Into<Cow<'a, str>>,
    replacement: &str,
) -> Cow<'a, str> {
    let mut result = None;
    let name = name.into();

    for (i, c) in name.char_indices() {
        if is_illegal_char(c) || is_control_char(c) {
            if result.is_none() {
                result.replace([&name[..i], replacement].concat());
            } else if let Some(ref mut s) = result {
                s.push_str(replacement);
            }
        } else if let Some(ref mut s) = result {
            s.push(c);
        }
    }

    result.map(Cow::Owned).unwrap_or(name)
}

fn is_reserved(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    name.chars().all(|c| c == '.')
}

fn is_windows_reserved(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let base = name.split_once(".").map(|(base, _)| base).unwrap_or(name);
    for &reserved in WINDOWS_RESERVED {
        if base.eq_ignore_ascii_case(reserved) {
            return true;
        }
    }

    false
}

fn has_windows_trailing(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    name.chars().last().map_or(false, |c| c == '.' || c == ' ')
}

fn replace_windows_trailing<'a>(
    name: impl Into<Cow<'a, str>>,
    replacement: &'a str,
) -> Cow<'a, str> {
    let name = name.into();
    let trimmed = name.trim_end_matches([' ', '.']);

    if trimmed.len() == name.len() {
        name
    } else if trimmed.is_empty() {
        replacement.into()
    } else {
        [trimmed, replacement].concat().into()
    }
}

#[derive(Clone)]
pub struct Options<'a> {
    pub windows: bool,
    pub truncate: bool,
    pub replacement: &'a str,
}

impl<'a> Default for Options<'a> {
    fn default() -> Self {
        Options {
            windows: cfg!(windows),
            truncate: true,
            replacement: "",
        }
    }
}

pub fn sanitize<'a, S: Into<Cow<'a, str>>>(name: S) -> Cow<'a, str> {
    sanitize_with_options(name, Options::default())
}

pub fn sanitize_with_options<'a, S: Into<Cow<'a, str>>>(
    name: S,
    Options {
        windows,
        truncate,
        replacement,
    }: Options<'a>,
) -> Cow<'a, str> {
    let name = name.into();
    let mut name = replace_illegal_or_control_char(name, replacement);

    if is_reserved(&name) {
        name = replacement.into();
    };

    if windows {
        name = replace_windows_trailing(name, replacement);

        if is_windows_reserved(&name) {
            name = replacement.into();
        }
    };

    if truncate && name.len() > 255 {
        let mut end = 255;
        while !name.is_char_boundary(end) {
            end -= 1;
        }

        match name {
            Cow::Borrowed(s) => {
                name = Cow::Borrowed(&s[..end]);
            }
            Cow::Owned(mut s) => {
                s.truncate(end);
                name = Cow::Owned(s);
            }
        }
    }

    name
}

#[derive(Clone)]
pub struct OptionsForCheck {
    pub windows: bool,
    pub truncate: bool,
}

impl Default for OptionsForCheck {
    fn default() -> Self {
        OptionsForCheck {
            windows: cfg!(windows),
            truncate: true,
        }
    }
}

pub fn is_sanitized<S: AsRef<str>>(name: S) -> bool {
    is_sanitized_with_options(name, OptionsForCheck::default())
}

pub fn is_sanitized_with_options<S: AsRef<str>>(
    name: S,
    OptionsForCheck { windows, truncate }: OptionsForCheck,
) -> bool {
    let name = name.as_ref();

    if name.is_empty() {
        return true;
    }

    if truncate && name.len() > 255 {
        return false;
    }

    if is_reserved(name) {
        return false;
    }

    if windows {
        if is_windows_reserved(name) {
            return false;
        }
        if has_windows_trailing(name) {
            return false;
        }
    }

    if name
        .chars()
        .any(|c| is_illegal_char(c) || is_control_char(c))
    {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // From https://github.com/parshap/node-sanitize-filename/blob/master/test.js
    static NAMES: &'static [&'static str] = &[
        "the quick brown fox jumped over the lazy dog",
        "résumé",
        "hello\u{0000}world",
        "hello\nworld",
        "semi;colon.js",
        ";leading-semi.js",
        "slash\\.js",
        "slash/.js",
        "col:on.js",
        "star*.js",
        "question?.js",
        "quote\".js",
        "singlequote'.js",
        "brack<e>ts.js",
        "p|pes.js",
        "plus+.js",
        "'five and six<seven'.js",
        " space at front",
        "space at end ",
        ".period",
        "period.",
        "relative/path/to/some/dir",
        "/abs/path/to/some/dir",
        "~/.\u{0000}notssh/authorized_keys",
        "",
        "h?w",
        "h/w",
        "h*w",
        ".",
        "..",
        "./",
        "../",
        "/..",
        "/../",
        "*.|.",
        "./",
        "./foobar",
        "../foobar",
        "../../foobar",
        "./././foobar",
        "|*.what",
        "LPT9.asdf",
        "foobar...",
    ];

    static NAMES_CLEANED: &'static [&'static str] = &[
        "the quick brown fox jumped over the lazy dog",
        "résumé",
        "helloworld",
        "helloworld",
        "semi;colon.js",
        ";leading-semi.js",
        "slash.js",
        "slash.js",
        "colon.js",
        "star.js",
        "question.js",
        "quote.js",
        "singlequote'.js",
        "brackets.js",
        "ppes.js",
        "plus+.js",
        "'five and sixseven'.js",
        " space at front",
        "space at end",
        ".period",
        "period",
        "relativepathtosomedir",
        "abspathtosomedir",
        "~.notsshauthorized_keys",
        "",
        "hw",
        "hw",
        "hw",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        ".foobar",
        "..foobar",
        "....foobar",
        "...foobar",
        ".what",
        "",
        "foobar",
    ];

    static NAMES_IS_SANITIZED: &'static [bool] = &[
        true, true, false, false, true, true, false, false, false, false, false, false, true,
        false, false, true, false, true, false, true, false, false, false, false, true, false,
        false, false, false, false, false, false, false, false, false, false, false, false, false,
        false, false, false, false,
    ];

    #[test]
    fn it_works() {
        // sanitize
        let options = Options {
            windows: true,
            truncate: true,
            replacement: "",
        };

        for (idx, &name) in NAMES.iter().enumerate() {
            assert_eq!(
                sanitize_with_options(name, options.clone()),
                NAMES_CLEANED[idx],
                "Failed at index {}: '{}' -> expected '{}', got '{}'",
                idx,
                name,
                NAMES_CLEANED[idx],
                sanitize_with_options(name, options.clone())
            );
        }

        let long = std::iter::repeat('a').take(300).collect::<String>();
        let shorter = std::iter::repeat('a').take(255).collect::<String>();
        assert_eq!(
            sanitize_with_options(long.as_str(), options.clone()),
            shorter
        );

        // is_sanitized
        let options = OptionsForCheck {
            windows: true,
            truncate: true,
        };

        for (idx, name) in NAMES.iter().enumerate() {
            assert_eq!(
                is_sanitized_with_options(name, options.clone()),
                NAMES_IS_SANITIZED[idx],
                "Failed at index {}: '{}' -> expected {}, got {}",
                idx,
                name,
                NAMES_IS_SANITIZED[idx],
                is_sanitized_with_options(name, options.clone())
            );
        }

        let long = std::iter::repeat('a').take(300).collect::<String>();
        assert_eq!(is_sanitized_with_options(long, options.clone()), false);
    }
}
