/// Bash compatibility database
///
/// Provides queryable interface to the feature compatibility data.
/// Easy to extend with new features and update support status as Rush evolves.

use super::features::{Feature, FeatureCategory, SupportStatus};

/// Database for querying bash feature compatibility
pub struct CompatDatabase;

impl CompatDatabase {
    /// Get summary statistics of feature support
    pub fn summary() -> CompatSummary {
        let features = super::features::all_features();

        let total = features.len();
        let supported = features
            .iter()
            .filter(|f| f.status == SupportStatus::Supported)
            .count();
        let planned = features
            .iter()
            .filter(|f| f.status == SupportStatus::Planned)
            .count();
        let not_supported = features
            .iter()
            .filter(|f| f.status == SupportStatus::NotSupported)
            .count();

        // Calculate by category
        let mut by_category = std::collections::HashMap::new();
        for category in &[
            FeatureCategory::Variables,
            FeatureCategory::ControlFlow,
            FeatureCategory::Builtins,
            FeatureCategory::Syntax,
            FeatureCategory::Expansions,
        ] {
            let cat_features = super::features::features_by_category(*category);
            by_category.insert(
                category.as_str(),
                CategorySummary {
                    total: cat_features.len(),
                    supported: cat_features
                        .iter()
                        .filter(|f| f.status == SupportStatus::Supported)
                        .count(),
                    planned: cat_features
                        .iter()
                        .filter(|f| f.status == SupportStatus::Planned)
                        .count(),
                    not_supported: cat_features
                        .iter()
                        .filter(|f| f.status == SupportStatus::NotSupported)
                        .count(),
                },
            );
        }

        CompatSummary {
            total,
            supported,
            planned,
            not_supported,
            support_percentage: if total > 0 {
                (supported * 100) / total
            } else {
                0
            },
            by_category,
        }
    }

    /// Find feature by ID
    pub fn find_feature(id: &str) -> Option<Feature> {
        super::features::get_feature(id)
    }

    /// Get all features in a category
    pub fn features_in_category(category: FeatureCategory) -> Vec<Feature> {
        super::features::features_by_category(category)
    }

    /// Get all supported features
    pub fn supported_features() -> Vec<Feature> {
        super::features::features_by_status(SupportStatus::Supported)
    }

    /// Get all planned features
    pub fn planned_features() -> Vec<Feature> {
        super::features::features_by_status(SupportStatus::Planned)
    }

    /// Get all unsupported features
    pub fn unsupported_features() -> Vec<Feature> {
        super::features::features_by_status(SupportStatus::NotSupported)
    }

    /// Get workarounds for unsupported feature
    pub fn get_workaround(feature_id: &str) -> Option<String> {
        super::features::get_feature(feature_id)
            .and_then(|f| {
                if f.status == SupportStatus::NotSupported {
                    f.workaround.map(|w| w.to_string())
                } else {
                    None
                }
            })
    }

    /// Check if a feature is supported
    pub fn is_supported(feature_id: &str) -> bool {
        super::features::get_feature(feature_id)
            .map(|f| f.status == SupportStatus::Supported)
            .unwrap_or(false)
    }

    /// Get migration guide (features not yet supported with workarounds)
    pub fn migration_guide() -> Vec<MigrationStep> {
        let unsupported = Self::unsupported_features();
        unsupported
            .iter()
            .filter_map(|f| {
                f.workaround.map(|w| MigrationStep {
                    feature_id: f.id.to_string(),
                    feature_name: f.name.to_string(),
                    bash_example: f.bash_example.to_string(),
                    workaround: w.to_string(),
                    category: f.category.as_str().to_string(),
                })
            })
            .collect()
    }

    /// Generate markdown documentation of all features
    pub fn to_markdown() -> String {
        let mut output = String::new();
        output.push_str("# Rush Bash Compatibility Database\n\n");

        let summary = Self::summary();
        output.push_str(&format!(
            "**Total Features:** {} | **Supported:** {} ({})% | **Planned:** {} | **Not Supported:** {}\n\n",
            summary.total, summary.supported, summary.support_percentage, summary.planned, summary.not_supported
        ));

        // Summary by category
        output.push_str("## Support by Category\n\n");
        output.push_str("| Category | Total | Supported | Planned | Not Supported |\n");
        output.push_str("|----------|-------|-----------|---------|---------------|\n");
        for (cat_name, cat_summary) in &summary.by_category {
            output.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                cat_name, cat_summary.total, cat_summary.supported, cat_summary.planned,
                cat_summary.not_supported
            ));
        }
        output.push('\n');

        // Features by category
        for category in &[
            FeatureCategory::Variables,
            FeatureCategory::ControlFlow,
            FeatureCategory::Builtins,
            FeatureCategory::Syntax,
            FeatureCategory::Expansions,
        ] {
            output.push_str(&format!("## {}\n\n", category.as_str()));

            let features = Self::features_in_category(*category);
            for feature in features {
                output.push_str(&format!(
                    "### {} [{}]\n\n",
                    feature.name, feature.status.as_str()
                ));
                output.push_str(&format!("**Description:** {}\n\n", feature.description));
                output.push_str(&format!(
                    "**Bash Example:** `{}`\n\n",
                    feature.bash_example
                ));
                output.push_str(&format!("**Bash Version:** {}\n\n", feature.bash_version));

                if let Some(rush_version) = feature.rush_version {
                    output.push_str(&format!(
                        "**Rush Support:** Added in {}\n\n",
                        rush_version
                    ));
                }

                if feature.status == SupportStatus::NotSupported {
                    if let Some(workaround) = feature.workaround {
                        output.push_str(&format!("**Workaround:** {}\n\n", workaround));
                    }
                }

                output.push_str(&format!("**Notes:** {}\n\n", feature.notes));
            }
        }

        output
    }
}

/// Summary statistics for compatibility
#[derive(Debug, Clone)]
pub struct CompatSummary {
    pub total: usize,
    pub supported: usize,
    pub planned: usize,
    pub not_supported: usize,
    pub support_percentage: usize,
    pub by_category: std::collections::HashMap<&'static str, CategorySummary>,
}

/// Category-specific summary
#[derive(Debug, Clone)]
pub struct CategorySummary {
    pub total: usize,
    pub supported: usize,
    pub planned: usize,
    pub not_supported: usize,
}

/// Migration guide step
#[derive(Debug, Clone)]
pub struct MigrationStep {
    pub feature_id: String,
    pub feature_name: String,
    pub bash_example: String,
    pub workaround: String,
    pub category: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_stats() {
        let summary = CompatDatabase::summary();
        assert_eq!(summary.total, summary.supported + summary.planned + summary.not_supported);
        assert!(summary.support_percentage <= 100);
    }

    #[test]
    fn test_find_feature() {
        let feature = CompatDatabase::find_feature("echo");
        assert!(feature.is_some());
        let f = feature.unwrap();
        assert_eq!(f.name, "Echo Builtin");
        assert_eq!(f.status, SupportStatus::Supported);
    }

    #[test]
    fn test_is_supported() {
        assert!(CompatDatabase::is_supported("echo"));
        assert!(CompatDatabase::is_supported("for-loop"));
        assert!(!CompatDatabase::is_supported("process-subst"));
    }

    #[test]
    fn test_supported_features() {
        let supported = CompatDatabase::supported_features();
        assert!(!supported.is_empty());
        assert!(supported.iter().all(|f| f.status == SupportStatus::Supported));
    }

    #[test]
    fn test_planned_features() {
        let planned = CompatDatabase::planned_features();
        assert!(!planned.is_empty());
        assert!(planned.iter().all(|f| f.status == SupportStatus::Planned));
    }

    #[test]
    fn test_unsupported_features() {
        let unsupported = CompatDatabase::unsupported_features();
        assert!(!unsupported.is_empty());
        assert!(unsupported.iter().all(|f| f.status == SupportStatus::NotSupported));
    }

    #[test]
    fn test_get_workaround() {
        let workaround = CompatDatabase::get_workaround("process-subst");
        assert!(workaround.is_some());

        let workaround = CompatDatabase::get_workaround("echo");
        assert!(workaround.is_none());
    }

    #[test]
    fn test_migration_guide() {
        let guide = CompatDatabase::migration_guide();
        assert!(!guide.is_empty());
        assert!(guide.iter().all(|s| !s.workaround.is_empty()));
    }

    #[test]
    fn test_features_in_category() {
        let variables = CompatDatabase::features_in_category(FeatureCategory::Variables);
        assert!(!variables.is_empty());
        assert!(variables.iter().all(|f| f.category == FeatureCategory::Variables));
    }

    #[test]
    fn test_markdown_generation() {
        let md = CompatDatabase::to_markdown();
        assert!(md.contains("# Rush Bash Compatibility Database"));
        assert!(md.contains("## variables"));
        assert!(md.contains("## control-flow"));
        assert!(md.contains("## builtins"));
        assert!(md.contains("## syntax"));
        assert!(md.contains("## expansions"));
    }
}
