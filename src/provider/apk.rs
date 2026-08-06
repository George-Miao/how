use super::{Detection, DetectionContext, Provider};
use crate::util;

pub(super) struct Apk;
pub(super) static PROVIDER: Apk = Apk;

impl Provider for Apk {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        util::query_ownership(context, "apk", &["info", "--who-owns"], "apk", parse)
    }
}

fn parse(output: &str) -> Option<String> {
    output
        .lines()
        .next()?
        .split(" is owned by ")
        .nth(1)?
        .split_whitespace()
        .next()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_owner() {
        assert_eq!(
            parse("/usr/bin/rg is owned by ripgrep-14.1-r0").as_deref(),
            Some("ripgrep-14.1-r0")
        );
    }
}
