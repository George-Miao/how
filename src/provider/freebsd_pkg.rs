use super::{Detection, DetectionContext, Provider};
use crate::util;

pub(super) struct FreeBsdPkg;
pub(super) static PROVIDER: FreeBsdPkg = FreeBsdPkg;

impl Provider for FreeBsdPkg {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        util::query_ownership(
            context,
            "pkg",
            &["which", "-q"],
            "FreeBSD pkg",
            util::first_line,
        )
    }
}
