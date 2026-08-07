use lazypackage_core::{Package, PackageStatus};

// ── Mode ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    Search,
    Confirm(ConfirmAction),
    /// Waiting for the user to type their sudo password before executing
    /// `pending_action`.
    SudoPrompt,
    ShowHelp,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    Install(String),
    Remove(String),
    Upgrade(String),
    UpgradeAll,
}

// ── Panel Focus ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum FocusedPanel {
    PackageList,
    Details,
    Log,
}

// ── Log Entry ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum LogEntry {
    Info(String),
    Success(String),
    Error(String),
    Command(String),
}

// ── App State ─────────────────────────────────────────────────────────────────

pub struct App {
    /// Full list of packages from the backend.
    pub packages: Vec<Package>,
    /// Indices into `packages` that pass the current filter.
    pub filtered_packages: Vec<usize>,
    /// Index within `filtered_packages` of the highlighted row.
    pub selected_idx: usize,
    /// First visible row (scroll position) in the package list.
    pub scroll_offset: usize,
    /// Current UI mode / overlay.
    pub mode: AppMode,
    /// Which panel currently has keyboard focus.
    pub focused_panel: FocusedPanel,
    /// Text currently typed in the search popup.
    pub search_query: String,
    /// Lines shown in the output log panel.
    pub log_entries: Vec<LogEntry>,
    /// Top-visible line of the log panel.
    pub log_scroll: usize,
    /// True while a backend operation is in-flight.
    pub is_loading: bool,
    /// Optional status filter (not exposed in keybindings yet, but available).
    pub status_filter: Option<PackageStatus>,
    /// Set to `true` to exit the event loop.
    pub should_quit: bool,
    /// Human-readable name of the active backend (e.g. `"dnf"`).
    pub backend_name: String,
    /// Text typed into the sudo password popup (masked on screen).
    pub sudo_input: String,
    /// The action waiting to be executed once the sudo password is confirmed.
    pub pending_action: Option<ConfirmAction>,
}

impl App {
    pub fn new(backend_name: String) -> Self {
        Self {
            packages: vec![],
            filtered_packages: vec![],
            selected_idx: 0,
            scroll_offset: 0,
            mode: AppMode::Normal,
            focused_panel: FocusedPanel::PackageList,
            search_query: String::new(),
            log_entries: vec![],
            log_scroll: 0,
            is_loading: false,
            status_filter: None,
            should_quit: false,
            backend_name,
            sudo_input: String::new(),
            pending_action: None,
        }
    }

    /// Replace the full package list and re-apply the current filter.
    pub fn set_packages(&mut self, packages: Vec<Package>) {
        self.packages = packages;
        self.apply_filter();
    }

    /// Rebuild `filtered_packages` from the current search query and status filter.
    /// Results are sorted: Installed → UpgradeAvailable → NotInstalled, then alphabetically.
    pub fn apply_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        self.filtered_packages = self
            .packages
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                let name_match = query.is_empty()
                    || p.name.to_lowercase().contains(&query)
                    || p.description.to_lowercase().contains(&query);
                let status_match = self.status_filter.map_or(true, |s| p.status == s);
                name_match && status_match
            })
            .map(|(i, _)| i)
            .collect();

        // Sort: Installed (0) → UpgradeAvailable (1) → NotInstalled (2), then name.
        self.filtered_packages.sort_by(|&a, &b| {
            let pa = &self.packages[a];
            let pb = &self.packages[b];
            let priority = |s: PackageStatus| match s {
                PackageStatus::Installed => 0u8,
                PackageStatus::UpgradeAvailable => 1,
                PackageStatus::NotInstalled => 2,
            };
            priority(pa.status)
                .cmp(&priority(pb.status))
                .then_with(|| pa.name.cmp(&pb.name))
        });

        // Clamp selection into valid range.
        if self.filtered_packages.is_empty() {
            self.selected_idx = 0;
            self.scroll_offset = 0;
        } else if self.selected_idx >= self.filtered_packages.len() {
            self.selected_idx = self.filtered_packages.len() - 1;
        }
    }

    /// Merges a second package list (e.g. all-available from repo) into `packages`.
    ///
    /// - If a package name already exists (installed), its entry is kept as-is
    ///   so the installed status is preserved.
    /// - New packages (not installed) are appended.
    /// - After merging, the filter is re-applied.
    pub fn merge_packages(&mut self, incoming: Vec<Package>) {
        let existing_names: std::collections::HashSet<String> =
            self.packages.iter().map(|p| p.name.clone()).collect();

        for pkg in incoming {
            if !existing_names.contains(&pkg.name) {
                self.packages.push(pkg);
            }
        }
        self.apply_filter();
    }

    /// Returns a reference to the currently highlighted package, if any.
    pub fn selected_package(&self) -> Option<&Package> {
        self.filtered_packages
            .get(self.selected_idx)
            .and_then(|&i| self.packages.get(i))
    }

    pub fn move_down(&mut self) {
        if !self.filtered_packages.is_empty() {
            self.selected_idx = (self.selected_idx + 1).min(self.filtered_packages.len() - 1);
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
        }
    }

    pub fn go_top(&mut self) {
        self.selected_idx = 0;
        self.scroll_offset = 0;
    }

    pub fn go_bottom(&mut self) {
        if !self.filtered_packages.is_empty() {
            self.selected_idx = self.filtered_packages.len() - 1;
        }
    }

    /// Append a log line and auto-scroll to the bottom.
    pub fn add_log(&mut self, entry: LogEntry) {
        self.log_entries.push(entry);
        self.log_scroll = self.log_entries.len().saturating_sub(1);
    }

    pub fn log_scroll_up(&mut self) {
        if self.log_scroll > 0 {
            self.log_scroll -= 1;
        }
    }

    pub fn log_scroll_down(&mut self) {
        if self.log_scroll + 1 < self.log_entries.len() {
            self.log_scroll += 1;
        }
    }

    /// Adjust `scroll_offset` so the selected row is always visible.
    pub fn update_scroll_offset(&mut self, visible_rows: usize) {
        let rows = visible_rows.max(1);
        if self.selected_idx < self.scroll_offset {
            self.scroll_offset = self.selected_idx;
        } else if self.selected_idx >= self.scroll_offset + rows {
            self.scroll_offset = self.selected_idx.saturating_sub(rows - 1);
        }
    }
}
