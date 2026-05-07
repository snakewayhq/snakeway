use crate::types::{DeviceSpec, DevicesFile, IngressFile, IngressSpec, OriginDeprecated};
use crate::validation::ConfigError;
use std::fs;
use std::path::Path;

pub(crate) fn parse_devices(path: &Path) -> Result<Vec<DeviceSpec>, ConfigError> {
    let s = fs::read_to_string(path).map_err(|e| ConfigError::read_file(path, e))?;
    let parsed: DevicesFile = hcl::from_str(&s).map_err(|e| ConfigError::parse(path, e))?;

    let mut device_config = Vec::new();

    if let Some(mut identity) = parsed.identity_device {
        identity.origin = OriginDeprecated::new(&path.to_path_buf(), "identity_device", None);
        device_config.push(DeviceSpec::Identity(identity));
    }

    if let Some(mut network_policy) = parsed.network_policy_device {
        network_policy.origin =
            OriginDeprecated::new(&path.to_path_buf(), "network_policy_device", None);
        device_config.push(DeviceSpec::NetworkPolicy(network_policy));
    }

    if let Some(mut request_rate_limiting) = parsed.request_rate_limiting_device {
        request_rate_limiting.origin =
            OriginDeprecated::new(&path.to_path_buf(), "request_rate_limiting_device", None);
        device_config.push(DeviceSpec::RequestRateLimiting(request_rate_limiting));
    }

    if let Some(mut logging) = parsed.structured_logging_device {
        logging.origin =
            OriginDeprecated::new(&path.to_path_buf(), "structured_logging_device", None);
        device_config.push(DeviceSpec::StructuredLogging(logging));
    }

    if let Some(mut request_filter) = parsed.request_filter_device {
        request_filter.origin =
            OriginDeprecated::new(&path.to_path_buf(), "request_filter_device", None);
        device_config.push(DeviceSpec::RequestFilter(request_filter));
    }

    for (idx, mut device) in parsed.wasm_devices.into_iter().enumerate() {
        device.origin = OriginDeprecated::new(&path.to_path_buf(), "wasm_device", idx.into());
        device_config.push(DeviceSpec::Wasm(device));
    }

    Ok(device_config)
}

pub(crate) fn parse_ingress(path: &Path) -> Result<IngressSpec, ConfigError> {
    let s = fs::read_to_string(path).map_err(|e| ConfigError::read_file(path, e))?;
    let mut parsed: IngressFile = hcl::from_str(&s).map_err(|e| ConfigError::parse(path, e))?;

    //-------------------------------------------------------------------------
    // Inject origin metadata
    //-------------------------------------------------------------------------
    if let Some(bind) = &mut parsed.bind {
        bind.origin = OriginDeprecated::new(&path.to_path_buf(), "bind", None);
    }

    if let Some(bind_admin) = &mut parsed.bind_admin {
        bind_admin.origin = OriginDeprecated::new(&path.to_path_buf(), "bind_admin", None);
    }

    for (i, service) in parsed.services.iter_mut().enumerate() {
        service.origin = OriginDeprecated::new(&path.to_path_buf(), "service", Some(i));
        for (j, route) in service.routes.iter_mut().enumerate() {
            route.origin = OriginDeprecated::new(&path.to_path_buf(), "route", Some(j));
        }
        for (j, backend) in service.upstreams.iter_mut().enumerate() {
            backend.origin = OriginDeprecated::new(&path.to_path_buf(), "backend", Some(j));
        }
    }

    for (i, static_files) in parsed.static_files.iter_mut().enumerate() {
        static_files.origin = OriginDeprecated::new(&path.to_path_buf(), "static_files", Some(i));
        for (j, route) in static_files.routes.iter_mut().enumerate() {
            route.origin = OriginDeprecated::new(&path.to_path_buf(), "route", Some(j));
        }
    }

    //-------------------------------------------------------------------------
    // Lower to ingress config
    //-------------------------------------------------------------------------

    Ok(IngressSpec {
        origin: OriginDeprecated::new(&path.to_path_buf(), "ingress", None),
        bind: parsed.bind,
        bind_admin: parsed.bind_admin,
        services: parsed.services,
        static_files: parsed.static_files,
    })
}

#[cfg(test)]
mod builtin_device_tests {
    use super::*;

    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parse_identity_device_file() {
        // Arrange
        let dir = tempdir().unwrap();
        let path = dir.path().join("identity.hcl");

        fs::write(
            &path,
            r#"
identity_device = {
  enable = true
  trusted_proxies = ["127.0.0.1/32"]
  enable_geoip = false
  enable_user_agent = false
  ua_engine = "woothee"
}
"#,
        )
        .unwrap();

        // Act
        let devices = parse_devices(&path).unwrap();

        // Assert
        assert_eq!(devices.len(), 1);
        assert!(matches!(devices[0], DeviceSpec::Identity(_)));
    }

    #[test]
    fn parse_structured_logging_device_file() {
        // Arrange
        let dir = tempdir().unwrap();
        let path = dir.path().join("structured_logging.hcl");

        fs::write(
            &path,
            r#"
structured_logging_device = {
  enable = true
  include_headers = false
  allowed_headers = []
  redacted_headers = []
  level = "info"
  include_identity = false
  identity_fields = []
}
"#,
        )
        .unwrap();

        // Act
        let devices = parse_devices(&path).unwrap();

        // Assert
        assert_eq!(devices.len(), 1);
        assert!(matches!(devices[0], DeviceSpec::StructuredLogging(_)));
    }
}

#[cfg(test)]
mod wasm_devices_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parse_wasm_device_array() {
        // Arrange
        let dir = tempdir().unwrap();
        let path = dir.path().join("wasm.hcl");

        fs::write(
            &path,
            r#"
wasm_devices = [
  { enable = false, path = "./a.wasm", config = {} },
  { enable = true,  path = "./b.wasm", config = {} }
]
"#,
        )
        .unwrap();

        // Act
        let devices = parse_devices(&path).unwrap();

        // Assert
        assert_eq!(devices.len(), 2);
        assert!(devices.iter().all(|d| matches!(d, DeviceSpec::Wasm(_))));
    }

    #[test]
    fn parse_devices_empty_file_is_ok() {
        // Arrange
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.hcl");

        fs::write(&path, "").unwrap();

        // Act
        let devices = parse_devices(&path).unwrap();

        // Assert
        assert!(devices.is_empty());
    }
}

#[cfg(test)]
mod ingress_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parse_ingress_bind_file() {
        // Arrange
        let dir = tempdir().unwrap();
        let path = dir.path().join("api.hcl");

        fs::write(
            &path,
            r#"
bind = {
  interface = "127.0.0.1"
  port = 8080
  enable_http2 = true
  tls = {
    mode = "manual"
    cert = "cert.pem"
    key  = "key.pem"
  }
}
"#,
        )
        .unwrap();

        // Act
        let ingress = parse_ingress(&path).unwrap();

        // Assert
        let bind = ingress.bind.unwrap();
        assert_eq!(bind.origin.section, "bind");
    }

    #[test]
    fn parse_ingress_admin_bind_file() {
        // Arrange
        let dir = tempdir().unwrap();
        let path = dir.path().join("admin.hcl");

        fs::write(
            &path,
            r#"
bind_admin = {
  interface = "127.0.0.1"
  port = 8080
  tls = {
    mode = "manual"
    cert = "cert.pem"
    key  = "key.pem"
  }
}
"#,
        )
        .unwrap();

        // Act
        let ingress = parse_ingress(&path).unwrap();

        // Assert
        let bind_admin = ingress.bind_admin.unwrap();
        assert_eq!(bind_admin.origin.section, "bind_admin");
    }

    #[test]
    fn parse_ingress_services_and_routes_have_origin() {
        // Arrange
        let dir = tempdir().unwrap();
        let path = dir.path().join("api.hcl");

        fs::write(
            &path,
            r#"
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
      { addr = "127.0.0.1:3000" }
    ]
  }
]
"#,
        )
        .unwrap();

        // Act
        let ingress = parse_ingress(&path).unwrap();

        // Assert
        let svc = &ingress.services[0];
        assert_eq!(svc.origin.section, "service");
        assert_eq!(svc.origin.index, Some(0));

        assert_eq!(svc.routes[0].origin.section, "route");
        assert_eq!(svc.routes[0].origin.index, Some(0));

        assert_eq!(svc.routes[1].origin.index, Some(1));
        assert_eq!(svc.upstreams[0].origin.section, "backend");
    }

    #[test]
    fn parse_ingress_invalid_hcl_returns_error() {
        // Arrange
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.hcl");

        fs::write(&path, "services = [").unwrap();

        // Act
        let err = parse_ingress(&path).unwrap_err();

        // Assert
        assert!(matches!(err, ConfigError::Parse { .. }));
    }
}
