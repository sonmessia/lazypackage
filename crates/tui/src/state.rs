use lazypackage_core::domain::{ActivePanel, Package, PackageId, PackageStatus, SearchScope};
use std::collections::HashSet;

pub struct AppState {
    pub installed_packages: Vec<Package>,
    pub dnf_search_results: Vec<Package>,
    pub search_scope: SearchScope,
    pub active_panel: ActivePanel,
    pub sidebar_index: usize,
    pub details_tab: usize,
    pub details_scroll: u16,
    pub log_scroll: u16,
    pub selected_package_index: Option<usize>,
    pub search_query: String,
    pub is_search_mode: bool,
    pub selected_packages: HashSet<PackageId>,
    pub log_messages: Vec<String>,
    pub error_message: Option<String>,
    pub is_loading: bool,
    pub current_category: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            installed_packages: Vec::new(),
            dnf_search_results: Vec::new(),
            search_scope: SearchScope::Local,
            active_panel: ActivePanel::PackageTable,
            sidebar_index: 0,
            details_tab: 0,
            details_scroll: 0,
            log_scroll: 0,
            selected_package_index: None,
            search_query: String::new(),
            is_search_mode: false,
            selected_packages: HashSet::new(),
            log_messages: Vec::new(),
            error_message: None,
            is_loading: false,
            current_category: "All".to_string(),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active_packages(&self) -> &Vec<Package> {
        match self.search_scope {
            SearchScope::Local => &self.installed_packages,
            SearchScope::Dnf => &self.dnf_search_results,
        }
    }

    pub fn filtered_packages(&self) -> Vec<&Package> {
        let source = self.active_packages();
        source
            .iter()
            .filter(|p| match self.current_category.as_str() {
                "Installed" => p.status() == PackageStatus::Installed,
                "Upgradable" => p.status() == PackageStatus::UpgradeAvailable,
                _ => true,
            })
            .filter(|p| {
                if self.search_query.trim().is_empty() {
                    true
                } else {
                    let query = self.search_query.to_lowercase();
                    p.id.name.to_lowercase().contains(&query)
                        || p.summary.to_lowercase().contains(&query)
                        || p.repo.as_deref().unwrap_or("").to_lowercase().contains(&query)
                        || p.installed_version.as_deref().unwrap_or("").to_lowercase().contains(&query)
                        || p.available_version.as_deref().unwrap_or("").to_lowercase().contains(&query)
                }
            })
            .collect()
    }

    pub fn selected_package(&self) -> Option<&Package> {
        let filtered = self.filtered_packages();
        self.selected_package_index.and_then(|idx| filtered.get(idx).copied())
    }

    pub fn clamp_selection(&mut self) {
        let count = self.filtered_packages().len();
        if count == 0 {
            self.selected_package_index = None;
        } else if let Some(idx) = self.selected_package_index {
            if idx >= count {
                self.selected_package_index = Some(count - 1);
            }
        } else {
            self.selected_package_index = Some(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazypackage_core::domain::BackendKind;

    fn mock_package(name: &str, summary: &str) -> Package {
        Package {
            id: PackageId {
                name: name.to_string(),
                backend: BackendKind::Dnf,
            },
            installed_version: Some("1.0.0".to_string()),
            available_version: None,
            size_bytes: Some(1024),
            repo: Some("fedora".to_string()),
            summary: summary.to_string(),
        }
    }

    #[test]
    fn test_search_filtering() {
        let mut state = AppState::new();
        state.installed_packages = vec![
            mock_package("git", "Fast version control system"),
            mock_package("vim", "Vi IMproved text editor"),
            mock_package("neovim", "Vim-fork focused on extensibility"),
        ];

        assert_eq!(state.filtered_packages().len(), 3);

        state.search_query = "vi".to_string();
        let filtered = state.filtered_packages();
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].id.name, "vim");
        assert_eq!(filtered[1].id.name, "neovim");

        state.search_query = "git".to_string();
        let filtered_git = state.filtered_packages();
        assert_eq!(filtered_git.len(), 1);
        assert_eq!(filtered_git[0].id.name, "git");
    }

    #[test]
    fn test_search_scopes() {
        let mut state = AppState::new();
        state.installed_packages = vec![mock_package("git", "Fast version control system")];
        state.dnf_search_results = vec![mock_package("htop", "Interactive process viewer")];

        state.search_scope = SearchScope::Local;
        assert_eq!(state.filtered_packages().len(), 1);
        assert_eq!(state.filtered_packages()[0].id.name, "git");

        state.search_scope = SearchScope::Dnf;
        assert_eq!(state.filtered_packages().len(), 1);
        assert_eq!(state.filtered_packages()[0].id.name, "htop");
    }

    #[test]
    fn test_clamp_selection() {
        let mut state = AppState::new();
        state.installed_packages = vec![
            mock_package("git", "Fast version control system"),
            mock_package("vim", "Vi IMproved text editor"),
        ];
        state.selected_package_index = Some(1);

        state.search_query = "git".to_string();
        state.clamp_selection();
        assert_eq!(state.selected_package_index, Some(0));
        assert_eq!(state.selected_package().unwrap().id.name, "git");

        state.search_query = "nonexistent".to_string();
        state.clamp_selection();
        assert_eq!(state.selected_package_index, None);
        assert!(state.selected_package().is_none());
    }

    #[test]
    fn test_active_panel_navigation() {
        let panel = ActivePanel::PackageTable;
        assert_eq!(panel.next(), ActivePanel::Details);
        assert_eq!(panel.next().next(), ActivePanel::Logs);
        assert_eq!(panel.next().next().next(), ActivePanel::Sidebar);
        assert_eq!(panel.prev(), ActivePanel::Sidebar);
    }
}
