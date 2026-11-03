use std::collections::VecDeque;

use backend::dock::DockWindowData;

#[derive(Debug, Default)]
pub(super) struct FocusRequests {
    observed: Option<u64>,
    pending: VecDeque<u64>,
}

impl FocusRequests {
    pub fn reconcile(&mut self, windows: &[DockWindowData]) {
        self.pending
            .retain(|id| windows.iter().any(|window| window.id == *id));
        let observed = windows
            .iter()
            .find(|window| window.is_focused)
            .map(|window| window.id);
        if observed == self.observed {
            return;
        }
        self.observed = observed;
        if let Some(index) = self.pending.iter().position(|id| Some(*id) == observed) {
            self.pending.drain(..=index);
        } else {
            self.pending.clear();
        }
    }

    pub fn request(&mut self, id: u64) {
        if self.pending.back() != Some(&id) {
            self.pending.push_back(id);
        }
    }

    pub fn next(&mut self, windows: &[DockWindowData], delta: i32) -> Option<u64> {
        self.reconcile(windows);
        if windows.is_empty() {
            return None;
        }
        let anchor = self.pending.back().copied().or(self.observed);
        let index = windows
            .iter()
            .position(|window| Some(window.id) == anchor)
            .unwrap_or(0);
        let next = (index as i64 + delta as i64).rem_euclid(windows.len() as i64) as usize;
        let id = windows[next].id;
        self.request(id);
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn windows(focused: u64) -> Vec<DockWindowData> {
        (1..=4)
            .map(|id| DockWindowData {
                id,
                workspace_id: Some(1),
                app_id: None,
                title: None,
                is_focused: id == focused,
                is_floating: false,
                is_urgent: false,
            })
            .collect()
    }

    #[test]
    fn rapid_scroll_keeps_later_requests_when_an_earlier_request_is_acknowledged() {
        let mut requests = FocusRequests::default();
        assert_eq!(requests.next(&windows(1), 1), Some(2));
        assert_eq!(requests.next(&windows(1), 1), Some(3));
        requests.reconcile(&windows(2));
        assert_eq!(requests.pending.back(), Some(&3));
        requests.reconcile(&windows(3));
        assert!(requests.pending.is_empty());
        assert_eq!(requests.next(&windows(1), 1), Some(2));
    }

    #[test]
    fn external_focus_and_clicks_replace_the_scroll_anchor() {
        let mut requests = FocusRequests::default();
        assert_eq!(requests.next(&windows(1), 1), Some(2));
        requests.reconcile(&windows(2));
        assert_eq!(requests.next(&windows(4), 1), Some(1));
        requests.request(3);
        assert_eq!(requests.next(&windows(4), 1), Some(4));
    }
}
