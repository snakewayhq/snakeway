use crate::conf::types::Origin;
use owo_colors::OwoColorize;
use serde::Serialize;
use std::fmt::Debug;

#[derive(Debug, Clone, Serialize)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub message: String,
    pub origin: Origin,
    pub help: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum Severity {
    Error,
    Warning,
}

pub struct ValidationReport {
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

#[derive(Serialize)]
struct ValidationReportJson<'a> {
    errors: &'a [ValidationIssue],
    warnings: &'a [ValidationIssue],
}

impl ValidationReport {
    pub fn has_violations(&self) -> bool {
        !self.errors.is_empty() || !self.warnings.is_empty()
    }

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

    pub fn render_json(&self) {
        let json = ValidationReportJson {
            errors: &self.errors,
            warnings: &self.warnings,
        };

        println!(
            "{}",
            serde_json::to_string_pretty(&json).expect("failed to serialize validation report")
        );
    }

    pub fn render_plain(&self) {
        for issue in self.errors.iter().chain(self.warnings.iter()) {
            let severity = match issue.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };

            println!(
                "{}:{}: {}",
                issue.origin.file.display(),
                severity,
                issue.message
            );

            if let Some(help) = &issue.help {
                println!("  help: {}", help);
            }
        }
    }

    pub fn render_pretty(&self) {
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
