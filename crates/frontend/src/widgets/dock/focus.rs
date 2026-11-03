use std::collections::VecDeque;

use backend::dock::{DockData, DockWindowData};

#[derive(Debug, PartialEq, Eq)]
struct FocusState {
    active_workspace: Option<u64>,
    focused_workspace: Option<u64>,
    focused_window: Option<u64>,
}

#[derive(Debug, Default)]
pub(super) struct FocusChangeTracker {
    previous: Option<FocusState>,
}

impl FocusChangeTracker {
    pub fn update(&mut self, data: &DockData) -> bool {
        let current = FocusState {
            active_workspace: data
                .workspaces
                .values()
                .find(|workspace| workspace.is_active)
                .map(|workspace| workspace.id),
            focused_workspace: data
                .workspaces
                .values()
                .find(|workspace| workspace.is_focused)
                .map(|workspace| workspace.id),
            focused_window: data
                .windows
                .iter()
                .find(|window| window.is_focused)
                .map(|window| window.id),
        };
        let should_pop = self.previous.as_ref().is_some_and(|previous| {
            (current.active_workspace.is_some()
                && current.active_workspace != previous.active_workspace)
                || (current.focused_workspace.is_some()
                    && current.focused_workspace != previous.focused_workspace)
                || (current.focused_window.is_some()
                    && current.focused_window != previous.focused_window)
        });
        self.previous = Some(current);
        should_pop
    }
}

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
    use backend::dock::DockWorkspaceData;

    fn data(active: u64, focused: bool, window: u64) -> DockData {
        DockData {
            windows: windows(window),
            workspaces: (1..=2)
                .map(|id| {
                    (
                        id,
                        DockWorkspaceData {
                            id,
                            idx: id,
                            name: None,
                            output: Some("eDP-1".into()),
                            is_active: id == active,
                            is_focused: focused && id == active,
                        },
                    )
                })
                .collect(),
        }
    }

    #[test]
    fn window_focus_changes_pop_but_initial_and_content_updates_do_not() {
        let mut tracker = FocusChangeTracker::default();
        let mut current = data(1, true, 1);
        assert!(!tracker.update(&current));
        current.windows[0].title = Some("Updated title".into());
        current.windows[0].is_urgent = true;
        current.windows.reverse();
        assert!(!tracker.update(&current));
        assert!(!tracker.update(&current));
        assert!(tracker.update(&data(1, true, 2)));
        assert!(tracker.update(&data(1, true, 3)));
        assert!(!tracker.update(&data(1, true, 3)));
    }

    #[test]
    fn switching_to_an_empty_workspace_pops() {
        let mut tracker = FocusChangeTracker::default();
        assert!(!tracker.update(&data(1, true, 1)));
        let mut empty = data(2, true, 0);
        empty.windows.clear();
        assert!(tracker.update(&empty));
        assert!(!tracker.update(&empty));
        assert!(tracker.update(&data(1, true, 1)));
    }

    #[test]
    fn gaining_output_focus_pops_even_without_windows_but_losing_it_does_not() {
        let mut tracker = FocusChangeTracker::default();
        let mut current = data(1, true, 0);
        current.windows.clear();
        assert!(!tracker.update(&current));
        current.workspaces.get_mut(&1).unwrap().is_focused = false;
        assert!(!tracker.update(&current));
        current.workspaces.get_mut(&1).unwrap().is_focused = true;
        assert!(tracker.update(&current));

        assert!(tracker.update(&data(1, true, 1)));
        assert!(!tracker.update(&data(1, true, 0)));
        assert!(!tracker.update(&data(1, false, 0)));
        assert!(!tracker.update(&data(1, false, 0)));
        assert!(tracker.update(&data(1, true, 1)));
    }

    #[test]
    fn workspace_renaming_and_reindexing_do_not_pop() {
        let mut tracker = FocusChangeTracker::default();
        let mut current = data(1, true, 1);
        assert!(!tracker.update(&current));
        let workspace = current.workspaces.get_mut(&1).unwrap();
        workspace.name = Some("Renamed".into());
        workspace.idx = 3;
        assert!(!tracker.update(&current));
    }

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
