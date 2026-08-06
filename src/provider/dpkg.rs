use super::{Detection, DetectionContext, Provider};
use crate::util;

pub(super) struct Dpkg;
pub(super) static PROVIDER: Dpkg = Dpkg;

impl Provider for Dpkg {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        util::query_ownership(context, "dpkg-query", &["-S"], "dpkg", parse)
    }
}

fn parse(output: &str) -> Option<String> {
    output
        .lines()
        .next()?
        .rsplit_once(':')
        .map(|(package, _)| package.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_owner() {
        assert_eq!(parse("ripgrep: /usr/bin/rg").as_deref(), Some("ripgrep"));
    }
}
