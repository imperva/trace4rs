use std::borrow::Cow;

use camino::{
    Utf8Path,
    Utf8PathBuf,
};
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

pub(crate) fn try_expand_env_vars(p: &Utf8Path) -> Cow<Utf8Path> {
    let expanded_str = expand_env_vars(p.as_str());
    match expanded_str {
        Cow::Borrowed(_) => Cow::Borrowed(p),
        Cow::Owned(o) => Cow::Owned(Utf8PathBuf::from(o)),
    }
}
