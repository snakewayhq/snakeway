use owo_colors::OwoColorize;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub message: String,
    pub file: PathBuf,
    pub help: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Severity {
    Error,
    Warning,
}

pub struct ValidationReport {
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn print(&self) {
        let errors = self
            .errors
            .iter()
            .filter(|i| matches!(i.severity, Severity::Error))
            .count();

        let warnings = self.warnings.len();

        if errors > 0 {
            println!(
                "configuration validation failed ({} errors, {} warnings)\n",
                errors, warnings
            );
        }

        let mut by_file = std::collections::BTreeMap::new();
        for issue in &self.errors {
            by_file.entry(&issue.file).or_insert(Vec::new()).push(issue);
        }

        for (file, issues) in by_file {
            println!("{}", file.display());

            for issue in issues {
                match issue.severity {
                    Severity::Error => {
                        println!("  {}: {}", "error".red().bold(), issue.message);
                    }
                    Severity::Warning => {
                        println!("  {}: {}", "warning".yellow().bold(), issue.message);
                    }
                }

                println!();
            }
        }
    }
}
