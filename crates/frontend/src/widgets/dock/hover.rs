use std::collections::HashMap;

#[derive(Debug, Default)]
pub(super) struct HoverState {
    from: HashMap<u64, f64>,
    target: Option<u64>,
}

impl HoverState {
    pub fn retarget(&mut self, target: Option<u64>, progress: f64) -> bool {
        if self.target == target {
            return false;
        }
        self.from = self
            .from
            .keys()
            .chain(self.target.iter())
            .map(|&id| (id, self.opacity(id, progress)))
            .filter(|(_, opacity)| *opacity > 0.0)
            .collect();
        self.target = target;
        true
    }

    pub fn opacity(&self, id: u64, progress: f64) -> f64 {
        let from = self.from.get(&id).copied().unwrap_or_default();
        let to = if self.target == Some(id) { 1.0 } else { 0.0 };
        from + (to - from) * progress.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_fades_in_and_out() {
        let mut hover = HoverState::default();
        assert_eq!(hover.opacity(1, 1.0), 0.0);
        assert!(hover.retarget(Some(1), 1.0));
        assert_eq!(hover.opacity(1, 0.0), 0.0);
        assert_eq!(hover.opacity(1, 0.5), 0.5);
        assert_eq!(hover.opacity(1, 1.0), 1.0);
        assert_eq!(hover.opacity(2, 1.0), 0.0);
        assert!(hover.retarget(None, 1.0));
        assert_eq!(hover.opacity(1, 0.0), 1.0);
        assert_eq!(hover.opacity(1, 0.5), 0.5);
        assert_eq!(hover.opacity(1, 1.0), 0.0);
    }

    #[test]
    fn rapid_hover_changes_continue_from_the_displayed_opacities() {
        let mut hover = HoverState::default();
        hover.retarget(Some(1), 1.0);
        assert!(hover.retarget(Some(2), 0.4));
        assert_eq!(hover.opacity(1, 0.0), 0.4);
        assert_eq!(hover.opacity(2, 0.0), 0.0);
        assert_eq!(hover.opacity(1, 0.5), 0.2);
        assert_eq!(hover.opacity(2, 0.5), 0.5);
        assert!(hover.retarget(Some(1), 0.5));
        assert_eq!(hover.opacity(1, 0.0), 0.2);
        assert_eq!(hover.opacity(2, 0.0), 0.5);
        assert_eq!(hover.opacity(1, 1.0), 1.0);
        assert_eq!(hover.opacity(2, 1.0), 0.0);
    }

    #[test]
    fn motion_within_the_same_item_does_not_restart_the_transition() {
        let mut hover = HoverState::default();
        assert!(!hover.retarget(None, 1.0));
        hover.retarget(Some(1), 1.0);
        assert!(!hover.retarget(Some(1), 0.3));
        assert_eq!(hover.opacity(1, 0.5), 0.5);
        hover.retarget(Some(2), 1.0);
        hover.retarget(None, 1.0);
        assert!(!hover.from.contains_key(&1));
        assert_eq!(hover.opacity(2, 1.0), 0.0);
    }

    #[test]
    fn completed_progress_applies_hover_changes_immediately() {
        let mut hover = HoverState::default();
        hover.retarget(Some(1), 1.0);
        assert_eq!(hover.opacity(1, 1.0), 1.0);
        hover.retarget(Some(2), 1.0);
        assert_eq!(hover.opacity(1, 1.0), 0.0);
        assert_eq!(hover.opacity(2, 1.0), 1.0);
        hover.retarget(None, 1.0);
        assert_eq!(hover.opacity(2, 1.0), 0.0);
    }
}
