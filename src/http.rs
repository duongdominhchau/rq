use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Unknown HTTP method: {0}")]
    UnknownMethod(String),
    #[error("Unknown Content-Type: {0}")]
    UnknownContentType(String),
}

// Need custom type because reqwest::Method allow arbitrary method.
/// The HTTP methods supported by this application
#[derive(Debug, Clone)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl FromStr for HttpMethod {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_uppercase().as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "PATCH" => HttpMethod::Patch,
            "HEAD" => HttpMethod::Head,
            "OPTIONS" => HttpMethod::Options,
            method => return Err(Error::UnknownMethod(method.to_string())),
        })
    }
}

impl Display for HttpMethod {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        };
        write!(f, "{}", str)
    }
}

impl From<HttpMethod> for reqwest::Method {
    fn from(m: HttpMethod) -> Self {
        match m {
            HttpMethod::Get => Self::GET,
            HttpMethod::Post => Self::POST,
            HttpMethod::Put => Self::PUT,
            HttpMethod::Delete => Self::DELETE,
            HttpMethod::Patch => Self::PATCH,
            HttpMethod::Head => Self::HEAD,
            HttpMethod::Options => Self::OPTIONS,
        }
    }
}

/// Possible body for a HttpMethod
#[derive(Debug, Clone)]
pub struct HttpBody {}
impl FromStr for HttpBody {
    type Err = Error;

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        Ok(HttpBody {})
    }
}

#[derive(Debug, Clone)]
pub enum ContentType {
    Text,
    Json,
    /// URL encoded (percent encoded)
    Form,
    Multipart,
}

impl FromStr for ContentType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "text" | "text/plain" => ContentType::Text,
            "json" | "application/json" => ContentType::Json,
            "form" | "application/x-www-form-urlencoded" => ContentType::Form,
            "file" | "multipart/form-data" => ContentType::Multipart,
            content_type => return Err(Error::UnknownContentType(content_type.to_string())),
        })
    }
}

impl Display for ContentType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match &self {
            ContentType::Text => "text/plain",
            ContentType::Json => "application/json",
            ContentType::Form => "application/x-www-form-urlencoded",
            ContentType::Multipart => "multipart/form-data",
        };
        write!(f, "{}", str)
    }
}

/// Guess whether the string is URL encoded (percent encoded) or not.
///
/// The guess is made based on whether a key followed by the equal sign can be found or not.
/// For example, this one is guessed to be URL encoded: `a-s.d_f~0+%21=`
///
/// Only some characters are allowed in the key, including the unreserved characters described in
/// https://datatracker.ietf.org/doc/html/rfc3986#section-2.3, the '+' (as some implementations
/// encode space to +), and the percent sign.
///
/// The percent sign is ensured to be followed by exactly two digits.
fn maybe_url_encoded(s: &str) -> bool {
    // The number of digits left to form a valid encoded byte.
    let mut digits_left = 0;
    #[allow(clippy::match_like_matches_macro)]
    let key_len = s
        .chars()
        .take_while(|c| {
            if digits_left > 0 {
                if let '0'..='9' = *c {
                    digits_left -= 1;
                    return true;
                } else {
                    return false;
                }
            }
            match c {
                // These characters can appear directly in the encoded string. Based on
                // https://datatracker.ietf.org/doc/html/rfc3986#section-2.3
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~' => true,
                // '+' is sometimes used for encoding whitespace
                '+' => true,
                '%' => {
                    digits_left = 2;
                    true
                }
                _ => false,
            }
        })
        .count();
    // No key found or the last byte is incomplete
    if key_len == 0 || digits_left > 0 {
        return false;
    }
    // If `key=` was found, it is probably URL encoded
    Some('=') == s.chars().nth(key_len)
}

macro_rules! match_or_stop_if_not_found_yet {
    ($actual_char:expr,$expected_char:expr,$flag:ident) => {
        if !$flag {
            if $actual_char == $expected_char {
                $flag = true;
                return true;
            } else {
                return false;
            }
        }
    };
}
/// Guess whether the string provided is JSON or not.
///
/// The guess is made based on the fact that JSON is usually sent with an object at the top level.
/// We check for a '{' followed by a string (for the key) then the colon ':'. Whitespaces are
/// ignored while checking for the pattern above. The key is assumed to have no escape sequence.
///
/// We also check if the string represents empty object if it is 20 bytes or shorter.
fn maybe_json(s: &str) -> bool {
    // If the string is short enough, we check if it is empty object
    if s.len() < 20 && s.chars().filter(|c| !c.is_whitespace()).collect::<String>() == "{}" {
        return true;
    }
    let mut open_bracket_found = false;
    let mut open_quote_found = false;
    let mut close_quote_found = false;
    let mut colon_found = false;
    // We don't need the result of count(), it is there just to trigger the execution
    s.chars()
        .take_while(|c| {
            if c.is_whitespace() {
                return true;
            }
            match_or_stop_if_not_found_yet!(*c, '{', open_bracket_found);
            match_or_stop_if_not_found_yet!(*c, '"', open_quote_found);
            // There can be almost anything inside a string. Here we assume that the key does not
            // contain escape sequence, so there is no special check for `*c == '\\'`
            if !close_quote_found {
                if *c == '"' {
                    close_quote_found = true;
                }
                return true;
            }
            match_or_stop_if_not_found_yet!(*c, ':', colon_found);
            // If it reach here, we have already matched the pattern, so we stop
            false
        })
        .count();
    // We guess that the string provided is JSON if we can find something like `{"key":`
    open_bracket_found && open_quote_found && close_quote_found && colon_found
}
/// Guess whether the content provided has multipart type.
///
/// I tried some tools to know what boundary they use, below is the result.
///
/// - Firefox 90
/// ```plain
/// -----------------------------<random>
/// ```
/// - Chromium 91:
/// ```plain
/// ------WebKitFormBoundary<random>
/// ```
/// - Postman 8.8:
/// ```plain
/// ----------------------------<random>
/// ```
/// - curl 7.77.0:
/// ```plain
/// --------------------------<random>
/// ```
///
/// Chromium has fewest number of hyphens (6). To be safe I will only check for 5 consecutive
/// hyphens to make the guess.
fn is_multipart(s: &str) -> bool {
    s.starts_with("-----")
}
/// Guess the content type of the content provided
pub fn guess_content_type(s: &str) -> ContentType {
    if maybe_json(s) {
        ContentType::Json
    } else if maybe_url_encoded(s) {
        ContentType::Form
    } else if is_multipart(s) {
        ContentType::Multipart
    } else {
        ContentType::Text
    }
}

// Tests having `looks_like` in name are about the case where the guess may be incorrect. Other
// tests are about the case where the guess is guaranteed to be correct. There are explanations
// in the specific tests.
#[cfg(test)]
mod tests {
    #[test]
    fn empty_string_does_not_look_like_url_encoded() {
        // Empty string is valid URL-encoded body, it's just not practical.
        assert!(!super::maybe_url_encoded(""));
    }
    #[test]
    fn str_without_eq_sign_does_not_look_like_url_encoded() {
        // Although we usually see the parameters in the form `key=value`, a parameter without
        // equal sign is valid
        assert!(!super::maybe_url_encoded("a-s.d_f~0+%21"));
    }
    #[test]
    fn str_with_incomplete_byte_is_not_url_encoded() {
        // This violates the URL encoded format, so this guess can't be wrong
        assert!(!super::maybe_url_encoded("%="));
        assert!(!super::maybe_url_encoded("%2="));
    }
    #[test]
    fn str_with_reserved_char_is_not_url_encoded() {
        // This one is also in wrong format
        assert!(!super::maybe_url_encoded("#a="));
        assert!(!super::maybe_url_encoded("&b="));
        assert!(!super::maybe_url_encoded("!c="));
        // Space should be encoded to `+` or `%20`, so the space here is clearly invalid
        assert!(!super::maybe_url_encoded(" hello=world"));
    }
    #[test]
    fn looks_like_url_encoded() {
        assert!(super::maybe_url_encoded("a-s.d_f~0+%21="));
        assert!(super::maybe_url_encoded("hello=world"));
    }
    #[test]
    fn str_with_nonobject_at_top_level_does_not_look_like_json() {
        // JSON does actually allow having non-object at top level, but object as top-level value
        // is more common
        assert!(!super::maybe_json("[1,2,3]"));
        assert!(!super::maybe_json(r#" "Hello" "#));
        assert!(!super::maybe_json(r#" 1 "#));
        assert!(!super::maybe_json(r#" 1.5 "#));
        assert!(!super::maybe_json(r#" false "#));
        assert!(!super::maybe_json(r#" null "#));
    }
    #[test]
    fn obj_with_nonterminated_key_is_not_json() {
        // This violates the spec, so it is clearly not valid JSON
        assert!(!super::maybe_json(r#"{"abcde"#));
        assert!(!super::maybe_json(r#"{"a:1}"#));
    }
    #[test]
    fn obj_with_nonstring_key_is_not_json() {
        // The spec requires the key to be of type string
        assert!(!super::maybe_json(r#"{1:"a"}"#));
        assert!(!super::maybe_json(r#"{true:"a"}"#));
        assert!(!super::maybe_json(r#"{null:"a"}"#));
        assert!(!super::maybe_json(r#"{'single':"a"}"#));
        assert!(!super::maybe_json(r#"{[1]:"a"}"#));
        assert!(!super::maybe_json(r#"{1.0:"a"}"#));
    }
    #[test]
    fn obj_having_key_without_colon_after_is_not_json() {
        assert!(!super::maybe_json(r#"{"a"}"#));
        assert!(!super::maybe_json(r#"{"a"=1}"#));
        assert!(!super::maybe_json(r#"{"a"<-1}"#));
    }
    #[test]
    fn empty_object_in_long_str_does_not_look_like_json() {
        // We have wrong guess here, but it should be fine, I have never seen this in practice
        assert!(!super::maybe_json("                    {}"));
    }
    #[test]
    fn empty_object_in_short_str_is_json() {
        assert!(super::maybe_json("{}"));
        assert!(super::maybe_json("     {}"));
        assert!(super::maybe_json("  {     }"));
        assert!(super::maybe_json("  {  }  "));
        assert!(super::maybe_json("{}        "));
    }
    #[test]
    fn looks_like_json() {
        assert!(super::maybe_json(r#"{"hello":"world"}"#));
        assert!(super::maybe_json(r#"{"a":{"b":{"c":{"d":"e"}}}}"#));
        // Another wrong guess for our heuristic
        assert!(super::maybe_json(r#"{"a":"#));
        // Empty string is still a string, so it is valid key
        assert!(super::maybe_json(r#"{"":0}"#));
    }
    #[test]
    fn spaces_in_json_does_not_matter() {
        assert!(super::maybe_json(
            r#"
        {
            "hello":        "world"
        }
            "#
        ));
        assert!(super::maybe_json(r#"  {"":  "#));
    }
}
