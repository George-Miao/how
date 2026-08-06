use super::{Detection, DetectionContext, Provider};
use crate::util;

pub(super) struct Pacman;
pub(super) static PROVIDER: Pacman = Pacman;

impl Provider for Pacman {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        util::query_ownership(context, "pacman", &["-Qqo"], "pacman", util::first_line)
    }
}
