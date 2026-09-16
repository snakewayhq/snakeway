use crate::proxy::TrafficProxy;
use pingora::prelude::HttpPeer;
use pingora::{BError, Custom, Error};
use snakeway_engine::ctx::RequestCtx;
use snakeway_engine::runtime::{RuntimeState, UpstreamRuntime, UpstreamTlsRuntime};
use snakeway_engine::traffic::{ProtocolMode, SelectedUpstream, ServiceId};

impl TrafficProxy {
    /// Select an upstream for the given request.
    pub(crate) fn select_upstream<'a>(
        &self,
        ctx: &RequestCtx,
        state: &'a RuntimeState,
        service_id: &ServiceId,
        service_name: &str,
    ) -> Result<SelectedUpstream<'a>, BError> {
        // Get a snapshot (cheap, lock-free)
        let snapshot = self.proxy_ctx.traffic_manager.snapshot();

        // Ask the director for a decision.
        let decision = self
            .traffic_director
            .decide(ctx, &snapshot, service_id, &self.proxy_ctx.traffic_manager)
            .map_err(|e| {
                tracing::error!(error = ?e, "traffic decision failed");
                Error::new(Custom("traffic decision failed"))
            })?;

        tracing::info!("decision reason: {}", decision.reason);

        // Grab the service by name.
        let service = state
            .services
            .get(service_name)
            .ok_or_else(|| Error::new(Custom("unknown service")))?;

        // Get the upstream based on the decision from the Traffic Director.
        let upstream = service
            .upstreams
            .iter()
            .find(|u| u.id() == decision.upstream_id)
            .ok_or_else(|| Error::new(Custom("selected upstream not found")))?;

        Ok(SelectedUpstream {
            upstream,
            cb_started: decision.cb_started,
        })
    }

    /// Build and configure the `HttpPeer` for the selected upstream.
    pub(in crate::proxy) fn build_peer(
        &self,
        ctx: &mut RequestCtx,
        upstream: &UpstreamRuntime,
    ) -> Result<HttpPeer, BError> {
        // Creating an HttpPeer instance per request may raise an eyebrow, but
        // it is merely a sort of configuration object that is used by Pingora
        // to compute a hash later when its internal pooling logic runs.
        let mut peer = match upstream {
            UpstreamRuntime::Tcp(tcp) => {
                let sni = tcp.tls.as_ref().map(|t| t.sni.clone()).unwrap_or_default();
                let mut peer = HttpPeer::new(tcp.http_peer_addr(), tcp.tls.is_some(), sni);
                if let Some(tls) = &tcp.tls {
                    apply_tls_verification(&mut peer, tls);
                }
                Ok(peer)
            }
            UpstreamRuntime::Unix(unix) => {
                let sni = unix.tls.as_ref().map(|t| t.sni.clone()).unwrap_or_default();
                HttpPeer::new_uds(&unix.path, unix.tls.is_some(), sni)
                    .map(|mut peer| {
                        if let Some(tls) = &unix.tls {
                            apply_tls_verification(&mut peer, tls);
                        }
                        peer
                    })
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Could not connect to unix domain socket `{}`: {}",
                            unix.path,
                            e
                        )
                    })
            }
        }
        .map_err(|_| Error::new(Custom("http peer creation failed")))?;

        // Apply upstream timeouts.
        // The read timeout is per-read (idle), so it bounds a stalled origin
        // without breaking slow-but-progressing responses.
        // It is skipped for websocket upgrades so idle long-lived connections
        // are not torn down.
        if let Some(t) = self.upstream_connect_timeout {
            // The total_connection_timeout setting bounds the whole connection
            // establishment (TCP connect and TLS handshake).
            // The inner connection_timeout (TCP connect only) is left unset
            // because it would be redundant since the total bound already caps it.
            peer.options.total_connection_timeout = Some(t);
        }
        if let Some(t) = self.upstream_read_timeout
            && !ctx.is_upgrade_req()
        {
            peer.options.read_timeout = Some(t);
        }

        // Resolve the wire protocol once and store it for later hooks.
        let mode = self.enforce_protocol(&mut peer, ctx, upstream)?;
        ctx.protocol_mode = Some(mode);

        // Set upstream authority for end-to-end h2 (gRPC, h2-to-h2).
        if mode == ProtocolMode::Http2EndToEnd {
            ctx.upstream_authority = Some(upstream.authority());
        }

        Ok(peer)
    }
}

/// Set how Pingora verifies the TLS certificate of an upstream peer.
fn apply_tls_verification(peer: &mut HttpPeer, tls: &UpstreamTlsRuntime) {
    peer.options.verify_cert = tls.verify;
    peer.options.verify_hostname = tls.verify;
    if tls.verify {
        peer.options.ca = tls.ca.clone();
        peer.group_key = tls.group_key;
    }
}
