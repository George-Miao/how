use super::{Detection, DetectionContext, Provider};
use crate::util;

pub(super) struct Rpm;
pub(super) static PROVIDER: Rpm = Rpm;

impl Provider for Rpm {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        util::query_ownership(
            context,
            "rpm",
            &["-qf", "--qf", "%{NAME}\\n"],
            "RPM",
            util::first_line,
        )
    }
}
