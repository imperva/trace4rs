use std::borrow::Cow;

use regex::Captures;

#[cfg(test)]
mod test;

#[allow(clippy::unwrap_used)] // ok since its a lit and tests hit this.
static RE: once_cell::sync::Lazy<regex::Regex> =
    once_cell::sync::Lazy::new(|| regex::Regex::new(r#"\$ENV\{([\w][\w|\d|\.|_]*)\}"#).unwrap());

pub(crate) fn expand_env_vars(path: &str) -> Cow<str> {
    RE.replace_all(path, |c: &Captures| {
        // For each capture there will be:
        // - 0: The entire match
        // - 1: The first and only group in that match
        #[allow(clippy::indexing_slicing)]
        if let Ok(s) = std::env::var(&c[1]) {
            s
        } else {
            c[0].to_string()
        }
    })
}
