use crate::types::HclOrigin;
use confval::{ValidationIssue, ValidationReport};
use owo_colors::OwoColorize;

pub fn render_json(report: &ValidationReport<HclOrigin>) {
    if !report.has_issues() {
        return;
    }

    #[derive(serde::Serialize)]
    struct IssueJson<'a> {
        severity: &'a str,
        message: &'a str,
        origin: &'a HclOrigin,
        help: &'a Option<String>,
    }

    #[derive(serde::Serialize)]
    struct ReportJson<'a> {
        errors: Vec<IssueJson<'a>>,
        warnings: Vec<IssueJson<'a>>,
    }

    fn to_json<'a>(issue: &'a ValidationIssue<HclOrigin>, severity: &'static str) -> IssueJson<'a> {
        IssueJson {
            severity,
            message: &issue.message,
            origin: &issue.origin,
            help: &issue.help,
        }
    }

    let json = ReportJson {
        errors: report
            .errors()
            .iter()
            .map(|i| to_json(i, "error"))
            .collect(),
        warnings: report
            .warnings()
            .iter()
            .map(|i| to_json(i, "warning"))
            .collect(),
    };

    match serde_json::to_string_pretty(&json) {
        Ok(output) => println!("{}", output),
        Err(e) => eprintln!("failed to serialize validation report: {}", e),
    }
}

pub fn render_plain(report: &ValidationReport<HclOrigin>) {
    if !report.has_issues() {
        return;
    }

    for issue in report.iter() {
        let severity = match issue.severity {
            confval::Severity::Error => "error",
            confval::Severity::Warning => "warning",
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

pub fn render_pretty(report: &ValidationReport<HclOrigin>) {
    if !report.has_issues() {
        return;
    }

    println!(
        "configuration validation failed ({} errors, {} warnings)\n",
        report.errors().len(),
        report.warnings().len()
    );

    let mut by_file = std::collections::BTreeMap::new();

    for issue in report.iter() {
        by_file
            .entry(&issue.origin.file)
            .or_insert(Vec::new())
            .push(issue);
    }

    for (file, issues) in by_file {
        println!("{}", file.display());

        for issue in issues {
            let help = issue
                .help
                .as_ref()
                .map(|h| format!("\n   help: {}", h))
                .unwrap_or_default();

            match issue.severity {
                confval::Severity::Error => {
                    println!("  {}: {}{}", "error".red().bold(), issue.message, help);
                }
                confval::Severity::Warning => {
                    println!("  {}: {}{}", "warning".yellow().bold(), issue.message, help);
                }
            }

            println!();
        }
    }
}
