use lazypackage_core::domain::{Package, PackageId};
use std::collections::HashSet;

pub struct AppState {
    pub packages: Vec<Package>,
    pub selected_package_index: Option<usize>,
    pub search_query: String,
    pub selected_packages: HashSet<PackageId>,
    pub log_messages: Vec<String>,
    pub error_message: Option<String>,
    pub is_loading: bool,
    pub current_category: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            packages: Vec::new(),
            selected_package_index: None,
            search_query: String::new(),
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
}
