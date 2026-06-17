use crate::types::{BindAdminSpec, BindSpec, ServiceSpec, StaticFilesSpec};
use confval::prelude::{Located, Report, Validate};
use serde::Serialize;

/// The operator DSL for the config subsystem.
/// This defines the configuration file format of files in ./config/ingress.d/*.hcl
#[derive(Debug, Serialize, Default, confval::Spec)]
#[serde(rename_all = "snake_case")]
pub struct IngressSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub bind: Option<Located<BindSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub bind_admin: Option<Located<BindAdminSpec>>,
    #[confval(nested)]
    pub services: Vec<Located<ServiceSpec>>,
    #[confval(nested)]
    pub static_files: Vec<Located<StaticFilesSpec>>,
}

/// Compositional field-local validation: an ingress validates itself by
/// delegating to each child entity. Delegating (rather than inlining) is what
/// makes the child validators reachable from the lowering bound on
/// `lower_configs` (`where IngressSpec: Validate`): removing any child's
/// `Validate` impl breaks this method, and the bound then fails to compile.
/// Checks needing an enclosing span (a missing child) stay in the central
/// validator, which holds the `Located` wrappers.
impl Validate for IngressSpec {
    fn validate(&self, report: &mut Report) {
        if let Some(bind) = &self.bind {
            bind.value.validate(report);
        }
        if let Some(bind_admin) = &self.bind_admin {
            bind_admin.value.validate(report);
        }
        for static_files in &self.static_files {
            static_files.value.validate(report);
        }
        for service in &self.services {
            service.value.validate(report);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TlsTerminationSpec;
    use confval::format::hcl::parse_hcl;
    use confval::prelude::{Report, SourceMap};

    fn parse(input: &str) -> (Report, Option<IngressSpec>) {
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("api.hcl", input);
        let spec = parse_hcl::<IngressSpec>(&sources, id, &mut report);
        (report, spec)
    }

    #[test]
    fn parse_bind_in_block_syntax() {
        // Arrange
        let input = r#"bind {
  interface = "127.0.0.1"
  port = 8080
  enable_http2 = true

  tls {
    mode = "manual"
    cert = "cert.pem"
    key  = "key.pem"
  }
}
"#;

        // Act
        let (report, spec) = parse(input);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let bind = spec.unwrap().bind.unwrap();
        assert_eq!(bind.value.interface.value, "127.0.0.1");
        assert_eq!(bind.value.port.value, 8080);
        assert!(bind.value.enable_http2.value);
        assert!(matches!(
            bind.value.tls.as_ref().unwrap().value,
            TlsTerminationSpec::Manual { .. }
        ));
    }

    #[test]
    fn parse_bind_in_object_syntax() {
        // Arrange
        let input = r#"bind = {
  interface = "127.0.0.1"
  port = 8080
  tls = {
    mode = "manual"
    cert = "cert.pem"
    key  = "key.pem"
  }
}
"#;

        // Act
        let (report, spec) = parse(input);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let bind = spec.unwrap().bind.unwrap();
        assert_eq!(bind.value.port.value, 8080);
        assert!(!bind.value.enable_http2.value, "defaults to false");
    }

    #[test]
    fn parse_services_as_array_of_objects_with_spans() {
        // Arrange
        let input = r#"bind = {
  interface = "loopback"
  port = 8080
}

services = [
  {
    routes = [
      {
        hosts = ["api.example.com"]
        path = "/api"
      },
      {
        hosts = ["ws.example.com"]
        path = "/ws"
      }
    ]

    upstreams = [
      { endpoint = { host = "127.0.0.1", port = 3000 } }
    ]
  }
]
"#;

        // Act
        let (report, spec) = parse(input);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let spec = spec.unwrap();
        let service = &spec.services[0];
        assert_eq!(service.value.routes.len(), 2);
        assert_eq!(service.value.routes[1].value.path.value, "/ws");
        assert_eq!(service.value.load_balancing_strategy.value, "failover");

        let path = &service.value.routes[1].value.path;
        assert_eq!(
            &input[path.span.start as usize..path.span.end as usize],
            "\"/ws\""
        );

        let upstream = &service.value.upstreams[0];
        assert_eq!(
            upstream.value.endpoint.as_ref().unwrap().value.host.value,
            "127.0.0.1"
        );
        assert_eq!(upstream.value.weight.value, 1, "weight defaults to 1");
    }

    #[test]
    fn parse_repeated_service_blocks() {
        // Arrange
        let input = r#"bind {
  interface = "loopback"
  port = 8080
}

service {
  routes = [{ hosts = ["a.example.com"], path = "/a" }]
  upstreams = [{ sock = "/tmp/a.sock" }]
}
"#;

        // Act: repeated blocks use the singular spelling only if the field
        // matches; this file uses `service` which is unknown for this spec.
        let (report, _) = parse(input);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "unknown block: service"),
            "issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn parse_unknown_field_is_reported() {
        // Arrange
        let input = "bnd = {\n  interface = \"loopback\"\n  port = 1\n}\n";

        // Act
        let (report, _) = parse(input);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "unknown field: bnd")
        );
    }

    #[test]
    fn parse_invalid_hcl_reports_syntax_error() {
        // Arrange
        let input = "services = [";

        // Act
        let (report, spec) = parse(input);

        // Assert
        assert!(spec.is_none());
        assert!(report.issues()[0].message.starts_with("syntax error:"));
    }
}
