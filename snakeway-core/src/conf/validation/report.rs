use crate::conf::types::Origin;
use owo_colors::OwoColorize;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub message: String,
    pub origin: Origin,
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
    pub fn error(&mut self, message: String, origin: &Origin, help: Option<String>) {
        self.errors.push(ValidationIssue {
            severity: Severity::Error,
            message,
            origin: origin.clone(),
            help,
        });
    }

    pub fn warning(&mut self, message: String, origin: &Origin, help: Option<String>) {
        self.warnings.push(ValidationIssue {
            severity: Severity::Warning,
            message,
            origin: origin.clone(),
            help,
        });
    }

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
            by_file
                .entry(&issue.origin.file)
                .or_insert(Vec::new())
                .push(issue);
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
