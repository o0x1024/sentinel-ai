use base64::{engine::general_purpose, Engine as _};
use brotli::Decompressor;
use bytes::Bytes;
use encoding_rs::{Encoding, UTF_8};
use flate2::read::{DeflateDecoder, GzDecoder};
use http::{uri::Authority, Request, Uri, Version};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::client::conn::http2;
use hyper_util::rt::{TokioExecutor, TokioIo};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Method, StatusCode,
};
use rustls::client::danger::{ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector as RustlsTlsConnector;
use url::Url;

const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawReplayRedirectHop {
    pub url: String,
    pub status_code: u16,
    pub location: Option<String>,
    pub set_cookie_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawReplayResult {
    pub raw_response: String,
    pub response_time_ms: u64,
    pub final_url: String,
    pub redirect_chain: Vec<RawReplayRedirectHop>,
    pub status_code: u16,
    pub version_observed: Option<String>,
    pub status_text: String,
    pub headers: Vec<ReplayHeaderInput>,
    pub body_text: String,
    pub body_bytes_base64: String,
}

#[derive(Debug, Clone)]
pub struct RawReplayConfig {
    pub endpoint: ReplayEndpointInput,
    pub request: ReplayRequestInput,
    pub timeout_secs: Option<u64>,
    pub follow_redirects: bool,
    pub max_redirects: usize,
    pub process_cookies_in_redirects: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayHeaderInput {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayEndpointInput {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub sni_host: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayRequestInput {
    pub method: String,
    pub target: String,
    pub version_preference: String,
    pub headers: Vec<ReplayHeaderInput>,
    pub body_text: String,
}

#[derive(Debug, Clone)]
struct ParsedRawRequest {
    method: String,
    target: String,
    protocol: String,
    headers: Vec<(String, String)>,
    body: String,
}

#[derive(Debug, Clone)]
struct ParsedRawResponse {
    protocol: Option<String>,
    status_code: u16,
    status_text: String,
    headers: Vec<(String, String)>,
    #[allow(dead_code)]
    body_text: String,
    location: Option<String>,
    set_cookie_headers: Vec<String>,
}

#[derive(Debug, Clone)]
struct RedirectCookie {
    name: String,
    value: String,
    domain: String,
    path: String,
    secure: bool,
    host_only: bool,
}

#[derive(Debug, Clone)]
struct ReplayDisplayResponse {
    raw_response: String,
    body_text: String,
    body_bytes_base64: String,
}

#[derive(Debug, Clone)]
struct ReplayTransportContext {
    connect_host: String,
    connect_port: u16,
    request_scheme: String,
    request_authority: String,
    tls_server_name: String,
}

#[derive(Debug)]
struct InsecureReplayServerCertVerifier;

enum ReplayHttp2Stream {
    Plain(TcpStream),
    Tls(tokio_rustls::client::TlsStream<TcpStream>),
}

impl ServerCertVerifier for InsecureReplayServerCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

impl AsyncRead for ReplayHttp2Stream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ReplayHttp2Stream::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            ReplayHttp2Stream::Tls(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl tokio::io::AsyncWrite for ReplayHttp2Stream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            ReplayHttp2Stream::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            ReplayHttp2Stream::Tls(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ReplayHttp2Stream::Plain(stream) => Pin::new(stream).poll_flush(cx),
            ReplayHttp2Stream::Tls(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ReplayHttp2Stream::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            ReplayHttp2Stream::Tls(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }

    fn is_write_vectored(&self) -> bool {
        match self {
            ReplayHttp2Stream::Plain(stream) => stream.is_write_vectored(),
            ReplayHttp2Stream::Tls(stream) => stream.is_write_vectored(),
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            ReplayHttp2Stream::Plain(stream) => Pin::new(stream).poll_write_vectored(cx, bufs),
            ReplayHttp2Stream::Tls(stream) => Pin::new(stream).poll_write_vectored(cx, bufs),
        }
    }
}

pub async fn replay_raw_request(config: RawReplayConfig) -> Result<RawReplayResult, String> {
    let timeout = std::time::Duration::from_secs(config.timeout_secs.unwrap_or(30));
    let start = std::time::Instant::now();

    let mut current_request = build_parsed_request(&config.request)?;
    let mut current_url = build_request_url(
        &current_request.target,
        &config.endpoint.host,
        config.endpoint.port,
        config.endpoint.scheme.eq_ignore_ascii_case("https"),
    )?;
    let mut cookie_jar = build_initial_cookie_jar(&current_request, &current_url);
    let mut redirect_chain = Vec::new();
    let mut last_display_response: Option<ReplayDisplayResponse> = None;
    let mut last_response_head: Option<ParsedRawResponse> = None;

    for redirect_index in 0..=config.max_redirects {
        let elapsed = start.elapsed();
        let remaining_timeout = timeout.saturating_sub(elapsed);
        if remaining_timeout.is_zero() {
            return Err("Total timeout exceeded".to_string());
        }

        let (display_response, used_http1_fallback) = execute_replay_request(
            &current_request,
            &current_url,
            &cookie_jar,
            &config.endpoint,
            remaining_timeout,
        )
        .await?;
        let response_head = parse_raw_response(&display_response.raw_response);
        last_display_response = Some(display_response);
        last_response_head = response_head.clone();

        if used_http1_fallback {
            current_request.protocol = "HTTP/1.1".to_string();
        }

        let Some(response_head) = response_head else {
            break;
        };

        let should_follow = config.follow_redirects
            && redirect_index < config.max_redirects
            && matches!(response_head.status_code, 301 | 302 | 303 | 307 | 308);

        let Some(location_header) = response_head.location.clone() else {
            break;
        };

        if !should_follow {
            break;
        }

        let next_url = current_url
            .join(&location_header)
            .map_err(|error| format!("Failed to resolve redirect URL: {error}"))?;

        if config.process_cookies_in_redirects {
            update_cookie_jar(
                &mut cookie_jar,
                &response_head.set_cookie_headers,
                &current_url,
            );
        }

        redirect_chain.push(RawReplayRedirectHop {
            url: current_url.to_string(),
            status_code: response_head.status_code,
            location: Some(next_url.to_string()),
            set_cookie_count: response_head.set_cookie_headers.len(),
        });

        current_request = build_redirect_request(
            &current_request,
            &current_url,
            &next_url,
            response_head.status_code,
        );
        current_url = next_url;
    }

    Ok(RawReplayResult {
        raw_response: last_display_response
            .as_ref()
            .map(|response| response.raw_response.clone())
            .unwrap_or_default(),
        response_time_ms: start.elapsed().as_millis() as u64,
        final_url: current_url.to_string(),
        redirect_chain,
        status_code: last_response_head
            .as_ref()
            .map(|response| response.status_code)
            .unwrap_or(0),
        version_observed: last_response_head
            .as_ref()
            .and_then(|response| response.protocol.clone()),
        status_text: last_response_head
            .as_ref()
            .map(|response| response.status_text.clone())
            .unwrap_or_default(),
        headers: last_response_head
            .as_ref()
            .map(|response| {
                response
                    .headers
                    .iter()
                    .map(|(name, value)| ReplayHeaderInput {
                        name: name.clone(),
                        value: value.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        body_text: last_display_response
            .as_ref()
            .map(|response| response.body_text.clone())
            .unwrap_or_default(),
        body_bytes_base64: last_display_response
            .as_ref()
            .map(|response| response.body_bytes_base64.clone())
            .unwrap_or_default(),
    })
}

async fn execute_replay_request(
    request: &ParsedRawRequest,
    target_url: &Url,
    cookie_jar: &[RedirectCookie],
    endpoint: &ReplayEndpointInput,
    timeout: std::time::Duration,
) -> Result<(ReplayDisplayResponse, bool), String> {
    let transport = build_replay_transport_context(request, target_url, endpoint)?;

    if is_http2_protocol(&request.protocol) {
        match execute_single_http2_request(request, target_url, cookie_jar, &transport, timeout)
            .await
        {
            Ok(raw_response) => return Ok((raw_response, false)),
            Err(error) if should_fallback_from_http2_to_http1(&error) => {
                tracing::warn!(
                    target = "sentinel::traffic::replay",
                    url = %target_url,
                    error = %error,
                    "HTTP/2 replay failed; falling back to HTTP/1.1"
                );
            }
            Err(error) => return Err(error),
        }
    }

    let http1_request = if is_http2_protocol(&request.protocol) {
        downgrade_request_to_http1(request)
    } else {
        request.clone()
    };
    let outbound_request = build_outbound_request(&http1_request, target_url, cookie_jar);
    let response_buf = execute_single_raw_request(
        &transport.connect_host,
        transport.connect_port,
        transport.request_scheme.eq_ignore_ascii_case("https"),
        &transport.tls_server_name,
        &outbound_request,
        timeout,
    )
    .await?;

    Ok((decode_http_response_parts(&response_buf), true))
}

fn build_replay_transport_context(
    request: &ParsedRawRequest,
    target_url: &Url,
    endpoint: &ReplayEndpointInput,
) -> Result<ReplayTransportContext, String> {
    let request_authority = build_effective_request_authority(request, target_url)?;
    let tls_server_name = endpoint
        .sni_host
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| extract_host_from_authority(&request_authority))
        .unwrap_or_else(|| endpoint.host.clone());

    Ok(ReplayTransportContext {
        connect_host: endpoint.host.clone(),
        connect_port: target_url.port_or_known_default().unwrap_or(endpoint.port),
        request_scheme: target_url.scheme().to_string(),
        request_authority,
        tls_server_name,
    })
}

fn build_effective_request_authority(
    request: &ParsedRawRequest,
    target_url: &Url,
) -> Result<String, String> {
    if let Some(host_header) = request
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("host"))
        .map(|(_, value)| value.trim())
        .filter(|value| !value.is_empty())
    {
        Authority::from_maybe_shared(host_header.to_string().into_bytes())
            .map_err(|error| format!("Invalid Host header for replay authority: {error}"))?;
        return Ok(host_header.to_string());
    }

    Ok(build_host_header(target_url))
}

fn extract_host_from_authority(authority: &str) -> Option<String> {
    Authority::from_maybe_shared(authority.trim().to_string().into_bytes())
        .ok()
        .map(|parsed| parsed.host().to_string())
}

fn downgrade_request_to_http1(request: &ParsedRawRequest) -> ParsedRawRequest {
    let mut downgraded = request.clone();
    downgraded.protocol = "HTTP/1.1".to_string();
    downgraded
}

fn build_parsed_request(request: &ReplayRequestInput) -> Result<ParsedRawRequest, String> {
    if request.method.trim().is_empty() {
        return Err("Replay request is missing the HTTP method".to_string());
    }

    if request.target.trim().is_empty() {
        return Err("Replay request is missing the request target".to_string());
    }

    Ok(ParsedRawRequest {
        method: request.method.trim().to_string(),
        target: request.target.trim().to_string(),
        protocol: normalize_http_version(&request.version_preference),
        headers: request
            .headers
            .iter()
            .filter(|header| !header.name.trim().is_empty())
            .map(|header| (header.name.trim().to_string(), header.value.clone()))
            .collect(),
        body: request.body_text.clone(),
    })
}

fn normalize_http_version(version: &str) -> String {
    match version.trim().to_ascii_uppercase().as_str() {
        "HTTP/1.0" => "HTTP/1.0".to_string(),
        "HTTP/2" | "HTTP/2.0" => "HTTP/2".to_string(),
        _ => "HTTP/1.1".to_string(),
    }
}

fn is_http2_protocol(protocol: &str) -> bool {
    matches!(
        protocol.trim().to_ascii_uppercase().as_str(),
        "HTTP/2" | "HTTP/2.0"
    )
}

fn should_fallback_from_http2_to_http1(error: &str) -> bool {
    let normalized = error.trim().to_ascii_lowercase();

    normalized.contains("failed to send http/2 request")
        || normalized.contains("http2 error")
        || normalized.contains("error sending request")
        || normalized.contains("connection closed")
        || normalized.contains("connection reset")
        || normalized.contains("broken pipe")
        || normalized.contains("tls")
        || normalized.contains("frame size")
        || normalized.contains("protocol error")
}

async fn execute_single_http2_request(
    request: &ParsedRawRequest,
    target_url: &Url,
    cookie_jar: &[RedirectCookie],
    transport: &ReplayTransportContext,
    timeout: std::time::Duration,
) -> Result<ReplayDisplayResponse, String> {
    let method = Method::from_bytes(request.method.as_bytes())
        .map_err(|error| format!("Unsupported HTTP method for HTTP/2 replay: {error}"))?;
    let request_uri = build_http2_request_uri(request, target_url, transport)?;
    let mut headers = build_http2_headers(request, target_url, cookie_jar)?;
    let authority_header = HeaderValue::from_str(&transport.request_authority)
        .map_err(|error| format!("Invalid HTTP/2 authority header: {error}"))?;
    headers.insert(HeaderName::from_static("host"), authority_header);

    let mut request_builder = Request::builder()
        .method(method)
        .uri(request_uri)
        .version(Version::HTTP_2);
    for (name, value) in headers.iter() {
        request_builder = request_builder.header(name, value);
    }

    let outbound_request = request_builder
        .body(Full::new(Bytes::from(request.body.clone())))
        .map_err(|error| format!("Failed to build HTTP/2 request: {error}"))?;

    let stream = connect_http2_stream(transport, timeout).await?;
    let io = TokioIo::new(stream);
    let (mut sender, connection) =
        tokio::time::timeout(timeout, http2::handshake(TokioExecutor::new(), io))
            .await
            .map_err(|_| "HTTP/2 handshake timeout".to_string())?
            .map_err(|error| format!("Failed to establish HTTP/2 session: {error}"))?;

    tokio::spawn(async move {
        if let Err(error) = connection.await {
            tracing::debug!(
                target = "sentinel::traffic::replay",
                error = %error,
                "HTTP/2 replay connection closed"
            );
        }
    });

    let response = tokio::time::timeout(timeout, sender.send_request(outbound_request))
        .await
        .map_err(|_| "HTTP/2 request timeout".to_string())?
        .map_err(|error| format!("Failed to send HTTP/2 request: {error}"))?;

    build_http2_raw_response(response).await
}

fn build_http2_headers(
    request: &ParsedRawRequest,
    target_url: &Url,
    cookie_jar: &[RedirectCookie],
) -> Result<HeaderMap, String> {
    let mut header_map = HeaderMap::new();

    for (name, value) in request.headers.iter() {
        if should_skip_http2_header(name, value) {
            continue;
        }

        let header_name = HeaderName::from_bytes(name.trim().as_bytes())
            .map_err(|error| format!("Invalid HTTP/2 header name `{name}`: {error}"))?;
        let header_value = HeaderValue::from_str(value.trim())
            .map_err(|error| format!("Invalid HTTP/2 header value for `{name}`: {error}"))?;
        header_map.append(header_name, header_value);
    }

    if let Some(cookie_header) = build_cookie_header(cookie_jar, target_url) {
        let header_value = HeaderValue::from_str(&cookie_header)
            .map_err(|error| format!("Invalid Cookie header for HTTP/2 replay: {error}"))?;
        header_map.insert(HeaderName::from_static("cookie"), header_value);
    } else {
        header_map.remove(HeaderName::from_static("cookie"));
    }

    Ok(header_map)
}

fn should_skip_http2_header(name: &str, value: &str) -> bool {
    let normalized = name.trim().to_ascii_lowercase();

    match normalized.as_str() {
        "host" | "content-length" | "connection" | "proxy-connection" | "keep-alive"
        | "transfer-encoding" | "upgrade" => true,
        "te" => !value.trim().eq_ignore_ascii_case("trailers"),
        _ => false,
    }
}

fn build_http2_request_uri(
    request: &ParsedRawRequest,
    target_url: &Url,
    transport: &ReplayTransportContext,
) -> Result<Uri, String> {
    let target = if request.target.starts_with("http://") || request.target.starts_with("https://")
    {
        Url::parse(&request.target)
            .map_err(|error| format!("Failed to parse HTTP/2 target URI: {error}"))?
    } else {
        let path = if request.target.starts_with('/') {
            request.target.clone()
        } else {
            format!("/{}", request.target)
        };
        target_url
            .join(&path)
            .map_err(|error| format!("Failed to resolve HTTP/2 target URI: {error}"))?
    };

    let mut path_and_query = target.path().to_string();
    if path_and_query.is_empty() {
        path_and_query.push('/');
    }
    if let Some(query) = target.query() {
        path_and_query.push('?');
        path_and_query.push_str(query);
    }

    let uri = format!(
        "{}://{}{}",
        transport.request_scheme, transport.request_authority, path_and_query
    );

    uri.parse::<Uri>()
        .map_err(|error| format!("Failed to build HTTP/2 URI: {error}"))
}

async fn connect_http2_stream(
    transport: &ReplayTransportContext,
    timeout: std::time::Duration,
) -> Result<ReplayHttp2Stream, String> {
    let tcp_stream = tokio::time::timeout(
        timeout,
        TcpStream::connect((transport.connect_host.as_str(), transport.connect_port)),
    )
    .await
    .map_err(|_| {
        format!(
            "Connection timeout to {}:{}",
            transport.connect_host, transport.connect_port
        )
    })?
    .map_err(|error| {
        format!(
            "Failed to connect to {}:{}: {}",
            transport.connect_host, transport.connect_port, error
        )
    })?;

    if !transport.request_scheme.eq_ignore_ascii_case("https") {
        return Ok(ReplayHttp2Stream::Plain(tcp_stream));
    }

    let mut tls_config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(InsecureReplayServerCertVerifier))
        .with_no_client_auth();
    tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    let tls_server_name =
        ServerName::try_from(transport.tls_server_name.clone()).map_err(|error| {
            format!(
                "Invalid TLS server name `{}`: {error}",
                transport.tls_server_name
            )
        })?;
    let connector = RustlsTlsConnector::from(Arc::new(tls_config));
    let tls_stream = tokio::time::timeout(timeout, connector.connect(tls_server_name, tcp_stream))
        .await
        .map_err(|_| "TLS handshake timeout".to_string())?
        .map_err(|error| format!("TLS handshake failed: {error}"))?;

    let negotiated = tls_stream
        .get_ref()
        .1
        .alpn_protocol()
        .map(|protocol| protocol.to_vec());
    if negotiated.as_deref() != Some(b"h2".as_slice()) {
        return Err(match negotiated {
            Some(protocol) => format!(
                "Failed to send HTTP/2 request: ALPN negotiated `{}` instead of `h2`",
                String::from_utf8_lossy(&protocol)
            ),
            None => "Failed to send HTTP/2 request: server did not negotiate ALPN".to_string(),
        });
    }

    Ok(ReplayHttp2Stream::Tls(tls_stream))
}

async fn build_http2_raw_response(
    response: hyper::Response<Incoming>,
) -> Result<ReplayDisplayResponse, String> {
    let status = response.status();
    let reason = canonical_reason(status);
    let mut response_head = if reason.is_empty() {
        format!("HTTP/2 {}\r\n", status.as_u16())
    } else {
        format!("HTTP/2 {} {}\r\n", status.as_u16(), reason)
    };

    for (name, value) in response.headers().iter() {
        if let Ok(value_str) = value.to_str() {
            response_head.push_str(name.as_str());
            response_head.push_str(": ");
            response_head.push_str(value_str);
            response_head.push_str("\r\n");
        }
    }

    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|error| format!("Failed to read HTTP/2 response body: {error}"))?;

    Ok(build_display_response_parts(
        &response_head,
        &body.to_bytes(),
        false,
    ))
}

fn canonical_reason(status: StatusCode) -> &'static str {
    status.canonical_reason().unwrap_or("")
}

async fn execute_single_raw_request(
    host: &str,
    port: u16,
    use_tls: bool,
    tls_server_name: &str,
    raw_request: &str,
    timeout: std::time::Duration,
) -> Result<Vec<u8>, String> {
    let stream = tokio::time::timeout(timeout, TcpStream::connect((host, port)))
        .await
        .map_err(|_| format!("Connection timeout to {host}:{port}"))?
        .map_err(|error| format!("Failed to connect to {host}:{port}: {error}"))?;

    if use_tls {
        let connector = tokio_native_tls::TlsConnector::from(
            native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true)
                .build()
                .map_err(|error| format!("Failed to create TLS connector: {error}"))?,
        );

        let mut tls_stream =
            tokio::time::timeout(timeout, connector.connect(tls_server_name, stream))
                .await
                .map_err(|_| "TLS handshake timeout".to_string())?
                .map_err(|error| format!("TLS handshake failed: {error}"))?;

        tls_stream
            .write_all(raw_request.as_bytes())
            .await
            .map_err(|error| format!("Failed to send request: {error}"))?;
        tls_stream
            .flush()
            .await
            .map_err(|error| format!("Failed to flush: {error}"))?;

        read_http_response(&mut tls_stream, timeout).await
    } else {
        let mut stream = stream;
        stream
            .write_all(raw_request.as_bytes())
            .await
            .map_err(|error| format!("Failed to send request: {error}"))?;
        stream
            .flush()
            .await
            .map_err(|error| format!("Failed to flush: {error}"))?;

        read_http_response(&mut stream, timeout).await
    }
}

#[cfg(test)]
fn parse_raw_request(raw_request: &str) -> Result<ParsedRawRequest, String> {
    let normalized = raw_request.replace("\r\n", "\n").replace('\r', "\n");
    let separator_index = normalized.find("\n\n").unwrap_or(normalized.len());
    let header_part = &normalized[..separator_index];
    let body = if separator_index < normalized.len() {
        normalized[separator_index + 2..].to_string()
    } else {
        String::new()
    };
    let mut lines = header_part.lines();

    let request_line = lines
        .next()
        .ok_or_else(|| "Raw request is missing the request line".to_string())?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| "Raw request is missing the HTTP method".to_string())?;
    let target = request_parts
        .next()
        .ok_or_else(|| "Raw request is missing the request target".to_string())?;
    let protocol = request_parts.next().unwrap_or("HTTP/1.1");

    let headers = lines
        .filter_map(|line| {
            let colon_index = line.find(':')?;
            Some((
                line[..colon_index].trim().to_string(),
                line[colon_index + 1..].trim().to_string(),
            ))
        })
        .collect();

    Ok(ParsedRawRequest {
        method: method.to_string(),
        target: target.to_string(),
        protocol: protocol.to_string(),
        headers,
        body,
    })
}

fn build_request_url(
    request_target: &str,
    host: &str,
    port: u16,
    use_tls: bool,
) -> Result<Url, String> {
    if request_target.starts_with("http://") || request_target.starts_with("https://") {
        return Url::parse(request_target)
            .map_err(|error| format!("Failed to parse request URL: {error}"));
    }

    let scheme = if use_tls { "https" } else { "http" };
    let default_port = if use_tls { 443 } else { 80 };
    let port_suffix = if port == default_port {
        String::new()
    } else {
        format!(":{port}")
    };
    let path = if request_target.starts_with('/') {
        request_target.to_string()
    } else {
        format!("/{request_target}")
    };

    Url::parse(&format!("{scheme}://{host}{port_suffix}{path}"))
        .map_err(|error| format!("Failed to build request URL: {error}"))
}

fn build_initial_cookie_jar(request: &ParsedRawRequest, request_url: &Url) -> Vec<RedirectCookie> {
    let mut jar = Vec::new();
    for (_, value) in request
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("cookie"))
    {
        for pair in value.split(';') {
            let trimmed = pair.trim();
            if trimmed.is_empty() {
                continue;
            }

            let Some((name, cookie_value)) = trimmed.split_once('=') else {
                continue;
            };

            upsert_cookie(
                &mut jar,
                RedirectCookie {
                    name: name.trim().to_string(),
                    value: cookie_value.trim().to_string(),
                    domain: request_url.host_str().unwrap_or_default().to_string(),
                    path: default_cookie_path(request_url),
                    secure: request_url.scheme() == "https",
                    host_only: true,
                },
            );
        }
    }
    jar
}

fn build_outbound_request(
    request: &ParsedRawRequest,
    target_url: &Url,
    cookie_jar: &[RedirectCookie],
) -> String {
    let mut headers = request.headers.clone();
    replace_or_insert_header(&mut headers, "Host", &build_host_header(target_url));

    let cookie_header = build_cookie_header(cookie_jar, target_url);
    match cookie_header {
        Some(value) => replace_or_insert_header(&mut headers, "Cookie", &value),
        None => remove_header(&mut headers, "Cookie"),
    }

    if request.body.is_empty() {
        remove_header(&mut headers, "Content-Length");
    } else {
        replace_or_insert_header(
            &mut headers,
            "Content-Length",
            &request.body.as_bytes().len().to_string(),
        );
    }

    let path = build_request_target(target_url);
    let mut raw_request = format!("{} {} {}\r\n", request.method, path, request.protocol);
    for (name, value) in headers {
        raw_request.push_str(&format!("{name}: {value}\r\n"));
    }
    raw_request.push_str("\r\n");
    raw_request.push_str(&request.body);
    raw_request
}

fn build_redirect_request(
    previous_request: &ParsedRawRequest,
    _current_url: &Url,
    next_url: &Url,
    status_code: u16,
) -> ParsedRawRequest {
    let preserve_method = matches!(status_code, 307 | 308);
    let mut request = previous_request.clone();

    if !preserve_method && !request.method.eq_ignore_ascii_case("HEAD") {
        request.method = "GET".to_string();
        request.body.clear();
    }

    request.target = build_request_target(next_url);
    request
}

fn parse_raw_response(raw_response: &str) -> Option<ParsedRawResponse> {
    let (header_end, separator_len) = if let Some(index) = raw_response.find("\r\n\r\n") {
        (index, 4)
    } else if let Some(index) = raw_response.find("\n\n") {
        (index, 2)
    } else {
        return None;
    };
    let header_part = &raw_response[..header_end];
    let body_text = raw_response[header_end + separator_len..].to_string();
    let mut lines = header_part.lines();
    let status_line = lines.next()?;
    let mut status_parts = status_line.split_whitespace();
    let protocol = status_parts
        .next()
        .and_then(normalize_observed_http_version);
    let status_code = status_parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())?;
    let status_text = status_parts.collect::<Vec<_>>().join(" ");

    let mut location = None;
    let mut set_cookie_headers = Vec::new();
    let mut headers = Vec::new();

    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let trimmed_value = value.trim().to_string();
        headers.push((name.trim().to_string(), trimmed_value.clone()));
        if name.eq_ignore_ascii_case("location") {
            location = Some(trimmed_value.clone());
        } else if name.eq_ignore_ascii_case("set-cookie") {
            set_cookie_headers.push(trimmed_value);
        }
    }

    Some(ParsedRawResponse {
        protocol,
        status_code,
        status_text,
        headers,
        body_text,
        location,
        set_cookie_headers,
    })
}

fn normalize_observed_http_version(protocol: &str) -> Option<String> {
    match protocol.trim().to_ascii_uppercase().as_str() {
        "HTTP/1.0" => Some("HTTP/1.0".to_string()),
        "HTTP/1.1" => Some("HTTP/1.1".to_string()),
        "HTTP/2" | "HTTP/2.0" => Some("HTTP/2".to_string()),
        _ => None,
    }
}

fn update_cookie_jar(
    cookie_jar: &mut Vec<RedirectCookie>,
    set_cookie_headers: &[String],
    request_url: &Url,
) {
    for header in set_cookie_headers {
        if let Some(cookie) = parse_set_cookie(header, request_url) {
            upsert_cookie(cookie_jar, cookie);
        }
    }
}

fn parse_set_cookie(header: &str, request_url: &Url) -> Option<RedirectCookie> {
    let mut parts = header.split(';');
    let name_value = parts.next()?.trim();
    let (name, value) = name_value.split_once('=')?;

    let mut cookie = RedirectCookie {
        name: name.trim().to_string(),
        value: value.trim().to_string(),
        domain: request_url.host_str()?.to_string(),
        path: default_cookie_path(request_url),
        secure: false,
        host_only: true,
    };

    for attribute in parts {
        let trimmed = attribute.trim();
        if trimmed.eq_ignore_ascii_case("secure") {
            cookie.secure = true;
            continue;
        }

        let Some((attr_name, attr_value)) = trimmed.split_once('=') else {
            continue;
        };

        if attr_name.eq_ignore_ascii_case("domain") {
            cookie.domain = attr_value.trim().trim_start_matches('.').to_string();
            cookie.host_only = false;
        } else if attr_name.eq_ignore_ascii_case("path") {
            cookie.path = normalize_cookie_path(attr_value.trim());
        }
    }

    Some(cookie)
}

fn upsert_cookie(cookie_jar: &mut Vec<RedirectCookie>, next_cookie: RedirectCookie) {
    cookie_jar.retain(|cookie| {
        !(cookie.name.eq_ignore_ascii_case(&next_cookie.name)
            && cookie.domain.eq_ignore_ascii_case(&next_cookie.domain)
            && cookie.path == next_cookie.path)
    });
    cookie_jar.push(next_cookie);
}

fn build_cookie_header(cookie_jar: &[RedirectCookie], target_url: &Url) -> Option<String> {
    let host = target_url.host_str().unwrap_or_default();
    let path = target_url.path();
    let is_secure = target_url.scheme() == "https";

    let values = cookie_jar
        .iter()
        .filter(|cookie| cookie_matches(cookie, host, path, is_secure))
        .map(|cookie| format!("{}={}", cookie.name, cookie.value))
        .collect::<Vec<_>>();

    if values.is_empty() {
        None
    } else {
        Some(values.join("; "))
    }
}

fn cookie_matches(cookie: &RedirectCookie, host: &str, path: &str, is_secure: bool) -> bool {
    if cookie.secure && !is_secure {
        return false;
    }

    let domain_matches = if cookie.host_only {
        cookie.domain.eq_ignore_ascii_case(host)
    } else {
        host.eq_ignore_ascii_case(&cookie.domain)
            || host
                .to_ascii_lowercase()
                .ends_with(&format!(".{}", cookie.domain.to_ascii_lowercase()))
    };

    if !domain_matches {
        return false;
    }

    path.starts_with(&cookie.path)
}

fn default_cookie_path(url: &Url) -> String {
    let path = url.path();
    if path.is_empty() || !path.starts_with('/') {
        return "/".to_string();
    }

    match path.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(index) => path[..index].to_string(),
    }
}

fn normalize_cookie_path(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

fn replace_or_insert_header(headers: &mut Vec<(String, String)>, name: &str, value: &str) {
    if let Some((_, existing_value)) = headers
        .iter_mut()
        .find(|(existing_name, _)| existing_name.eq_ignore_ascii_case(name))
    {
        *existing_value = value.to_string();
        return;
    }

    headers.push((name.to_string(), value.to_string()));
}

fn remove_header(headers: &mut Vec<(String, String)>, name: &str) {
    headers.retain(|(existing_name, _)| !existing_name.eq_ignore_ascii_case(name));
}

fn build_host_header(target_url: &Url) -> String {
    let host = target_url.host_str().unwrap_or_default();
    let default_port = match target_url.scheme() {
        "https" => 443,
        _ => 80,
    };

    match target_url.port() {
        Some(port) if port != default_port => format!("{host}:{port}"),
        _ => host.to_string(),
    }
}

fn build_request_target(target_url: &Url) -> String {
    let mut target = target_url.path().to_string();
    if target.is_empty() {
        target.push('/');
    }
    if let Some(query) = target_url.query() {
        target.push('?');
        target.push_str(query);
    }
    target
}

fn decode_chunked(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        let line_end = data[pos..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .map(|offset| pos + offset);

        let Some(line_end) = line_end else {
            break;
        };

        let size_str = String::from_utf8_lossy(&data[pos..line_end]);
        let size_str = size_str.split(';').next().unwrap_or("").trim();
        let chunk_size = match usize::from_str_radix(size_str, 16) {
            Ok(size) => size,
            Err(_) => break,
        };

        if chunk_size == 0 {
            break;
        }

        let chunk_start = line_end + 2;
        let chunk_end = chunk_start + chunk_size;
        if chunk_end > data.len() {
            break;
        }

        result.extend_from_slice(&data[chunk_start..chunk_end]);
        pos = chunk_end + 2;
    }

    if result.is_empty() {
        data.to_vec()
    } else {
        result
    }
}

fn parse_content_encodings(response_head: &str) -> Vec<String> {
    response_head
        .lines()
        .find(|line| line.to_lowercase().starts_with("content-encoding:"))
        .map(|line| {
            line.split_once(':')
                .map(|(_, value)| {
                    value
                        .split(',')
                        .map(|encoding| encoding.trim().to_lowercase())
                        .filter(|encoding| !encoding.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .unwrap_or_default()
}

fn parse_charset_from_content_type(response_head: &str) -> Option<String> {
    response_head
        .lines()
        .find(|line| line.to_lowercase().starts_with("content-type:"))
        .and_then(|line| line.split_once(':').map(|(_, value)| value))
        .and_then(|value| {
            value.split(';').skip(1).find_map(|part| {
                let (name, charset) = part.split_once('=')?;
                if name.trim().eq_ignore_ascii_case("charset") {
                    let normalized = charset.trim().trim_matches('"').trim_matches('\'');
                    if normalized.is_empty() {
                        None
                    } else {
                        Some(normalized.to_string())
                    }
                } else {
                    None
                }
            })
        })
}

fn decode_content_encoded_body(body_bytes: &[u8], encodings: &[String]) -> Vec<u8> {
    let mut decoded_body = body_bytes.to_vec();

    for encoding in encodings.iter().rev() {
        let next_body = match encoding.as_str() {
            "gzip" | "x-gzip" => {
                let mut decoder = GzDecoder::new(decoded_body.as_slice());
                let mut decoded = Vec::new();
                match decoder.read_to_end(&mut decoded) {
                    Ok(_) => decoded,
                    Err(_) => return body_bytes.to_vec(),
                }
            }
            "deflate" => {
                let mut decoder = DeflateDecoder::new(decoded_body.as_slice());
                let mut decoded = Vec::new();
                match decoder.read_to_end(&mut decoded) {
                    Ok(_) => decoded,
                    Err(_) => return body_bytes.to_vec(),
                }
            }
            "br" => {
                let mut decoder = Decompressor::new(decoded_body.as_slice(), 4096);
                let mut decoded = Vec::new();
                match decoder.read_to_end(&mut decoded) {
                    Ok(_) => decoded,
                    Err(_) => return body_bytes.to_vec(),
                }
            }
            "identity" => decoded_body,
            _ => return body_bytes.to_vec(),
        };
        decoded_body = next_body;
    }

    decoded_body
}

fn decode_body_text(body_bytes: &[u8], response_head: &str) -> String {
    let charset = parse_charset_from_content_type(response_head)
        .and_then(|label| Encoding::for_label(label.as_bytes()))
        .unwrap_or(UTF_8);

    let (decoded, _, _) = charset.decode(body_bytes);
    decoded.into_owned()
}

fn build_display_response_parts(
    response_head: &str,
    body_bytes: &[u8],
    decode_chunked_body: bool,
) -> ReplayDisplayResponse {
    let header_lower = response_head.to_lowercase();
    let body_bytes = if decode_chunked_body
        && header_lower
            .lines()
            .any(|line| line.starts_with("transfer-encoding:") && line.contains("chunked"))
    {
        decode_chunked(body_bytes)
    } else {
        body_bytes.to_vec()
    };

    let decoded_body =
        decode_content_encoded_body(&body_bytes, &parse_content_encodings(response_head));
    let body_text = decode_body_text(&decoded_body, response_head);
    let mut raw_response = response_head.to_string();
    if response_head.ends_with("\r\n") {
        raw_response.push_str("\r\n");
    } else {
        raw_response.push_str("\r\n\r\n");
    }
    raw_response.push_str(&body_text);

    ReplayDisplayResponse {
        raw_response,
        body_text,
        body_bytes_base64: general_purpose::STANDARD.encode(decoded_body),
    }
}

#[cfg(test)]
fn build_display_response(
    response_head: &str,
    body_bytes: &[u8],
    decode_chunked_body: bool,
) -> String {
    build_display_response_parts(response_head, body_bytes, decode_chunked_body).raw_response
}

fn decode_http_response_parts(response_buf: &[u8]) -> ReplayDisplayResponse {
    let header_end = response_buf
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|offset| offset + 4);

    let Some(header_end) = header_end else {
        let raw_response = String::from_utf8_lossy(response_buf).to_string();
        return ReplayDisplayResponse {
            body_text: raw_response.clone(),
            body_bytes_base64: general_purpose::STANDARD.encode(response_buf),
            raw_response,
        };
    };

    let header_bytes = &response_buf[..header_end];
    let body_bytes = &response_buf[header_end..];
    let response_head = String::from_utf8_lossy(header_bytes)
        .trim_end_matches("\r\n\r\n")
        .to_string();

    build_display_response_parts(&response_head, body_bytes, true)
}

async fn read_http_response<S: AsyncRead + Unpin>(
    stream: &mut S,
    total_timeout: std::time::Duration,
) -> Result<Vec<u8>, String> {
    let mut response_buf = Vec::new();
    let mut buf = [0u8; 8192];
    let mut headers_parsed = false;
    let mut content_length: Option<usize> = None;
    let mut is_chunked = false;
    let mut header_end_pos: Option<usize> = None;
    let read_timeout = std::time::Duration::from_millis(500);
    let start_time = std::time::Instant::now();

    loop {
        if start_time.elapsed() >= total_timeout {
            return Err("Total timeout exceeded".to_string());
        }

        if response_buf.len() > MAX_RESPONSE_SIZE {
            return Err(format!(
                "Response too large: {} bytes (max: {} bytes)",
                response_buf.len(),
                MAX_RESPONSE_SIZE
            ));
        }

        match tokio::time::timeout(read_timeout, stream.read(&mut buf)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => {
                response_buf.extend_from_slice(&buf[..n]);

                if !headers_parsed {
                    if let Some(pos) = response_buf
                        .windows(4)
                        .position(|window| window == b"\r\n\r\n")
                    {
                        header_end_pos = Some(pos + 4);
                        headers_parsed = true;

                        let header_str = String::from_utf8_lossy(&response_buf[..pos]);
                        for line in header_str.lines() {
                            let lower = line.to_lowercase();
                            if lower.starts_with("content-length:") {
                                if let Some(length) = line
                                    .split(':')
                                    .nth(1)
                                    .and_then(|value| value.trim().parse::<usize>().ok())
                                {
                                    content_length = Some(length);
                                }
                            } else if lower.starts_with("transfer-encoding:")
                                && lower.contains("chunked")
                            {
                                is_chunked = true;
                            }
                        }
                    }
                }

                if headers_parsed {
                    if let Some(header_end) = header_end_pos {
                        let body_len = response_buf.len().saturating_sub(header_end);
                        if let Some(expected_length) = content_length {
                            if body_len >= expected_length {
                                break;
                            }
                        }

                        if is_chunked && response_buf.len() >= 5 {
                            let tail = &response_buf[response_buf.len().saturating_sub(7)..];
                            if tail.windows(5).any(|window| window == b"0\r\n\r\n") {
                                break;
                            }
                        }
                    }
                }
            }
            Ok(Err(error)) => return Err(format!("Read error: {error}")),
            Err(_) => {
                if headers_parsed {
                    break;
                }
                if response_buf.is_empty() && start_time.elapsed() < total_timeout {
                    continue;
                }
                break;
            }
        }
    }

    Ok(response_buf)
}

#[cfg(test)]
mod tests {
    use super::{
        build_display_response, build_effective_request_authority, build_http2_request_uri,
        build_initial_cookie_jar, build_outbound_request, build_redirect_request,
        build_replay_transport_context, build_request_url, cookie_matches,
        downgrade_request_to_http1, is_http2_protocol, parse_raw_request, parse_raw_response,
        parse_set_cookie, should_fallback_from_http2_to_http1, should_skip_http2_header,
        RedirectCookie,
    };
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;
    use url::Url;

    #[test]
    fn parses_set_cookie_attributes() {
        let url = Url::parse("https://example.com/app/login").unwrap();
        let cookie =
            parse_set_cookie("session=abc; Path=/app; Secure; Domain=example.com", &url).unwrap();

        assert_eq!(cookie.name, "session");
        assert_eq!(cookie.path, "/app");
        assert!(cookie.secure);
        assert_eq!(cookie.domain, "example.com");
        assert!(!cookie.host_only);
    }

    #[test]
    fn cookie_matching_respects_domain_path_and_scheme() {
        let cookie = RedirectCookie {
            name: "session".to_string(),
            value: "abc".to_string(),
            domain: "example.com".to_string(),
            path: "/app".to_string(),
            secure: true,
            host_only: false,
        };

        assert!(cookie_matches(
            &cookie,
            "sub.example.com",
            "/app/dashboard",
            true
        ));
        assert!(!cookie_matches(&cookie, "sub.example.com", "/admin", true));
        assert!(!cookie_matches(
            &cookie,
            "sub.example.com",
            "/app/dashboard",
            false
        ));
    }

    #[test]
    fn redirect_rewrites_post_to_get_for_302() {
        let request = parse_raw_request(
            "POST /login HTTP/1.1\r\nHost: example.com\r\nContent-Length: 3\r\n\r\nx=1",
        )
        .unwrap();
        let next_url = Url::parse("https://example.com/home").unwrap();
        let redirected = build_redirect_request(&request, &next_url, &next_url, 302);

        assert_eq!(redirected.method, "GET");
        assert!(redirected.body.is_empty());
        assert_eq!(redirected.target, "/home");
    }

    #[test]
    fn outbound_request_preserves_form_body_without_trailing_crlf() {
        let raw_request = concat!(
            "POST /cart HTTP/1.1\r\n",
            "Host: example.com\r\n",
            "Content-Type: application/x-www-form-urlencoded\r\n",
            "Content-Length: 999\r\n",
            "\r\n",
            "productId=1&redir=PRODUCT&quantity=1&price=133700"
        );
        let request = parse_raw_request(raw_request).unwrap();
        let target_url = build_request_url(&request.target, "example.com", 443, true).unwrap();
        let cookie_jar = build_initial_cookie_jar(&request, &target_url);

        let outbound = build_outbound_request(&request, &target_url, &cookie_jar);

        assert!(outbound.ends_with("productId=1&redir=PRODUCT&quantity=1&price=133700"));
        assert!(!outbound.ends_with("\r\n"));
        assert!(outbound.contains("\r\nContent-Length: 49\r\n"));
    }

    #[test]
    fn detects_http2_protocol_tokens() {
        assert!(is_http2_protocol("HTTP/2"));
        assert!(is_http2_protocol("http/2.0"));
        assert!(!is_http2_protocol("HTTP/1.1"));
    }

    #[test]
    fn filters_connection_specific_headers_for_http2() {
        assert!(should_skip_http2_header("Connection", "keep-alive"));
        assert!(should_skip_http2_header("Transfer-Encoding", "chunked"));
        assert!(should_skip_http2_header("TE", "gzip"));
        assert!(!should_skip_http2_header("TE", "trailers"));
        assert!(!should_skip_http2_header("Accept", "*/*"));
    }

    #[test]
    fn detects_http2_errors_that_should_fallback() {
        assert!(should_fallback_from_http2_to_http1(
            "Failed to send HTTP/2 request: error sending request for url (https://woa.wps.cn/api/v2/contacts)"
        ));
        assert!(should_fallback_from_http2_to_http1(
            "Failed to send HTTP/2 request: connection closed before message completed"
        ));
        assert!(!should_fallback_from_http2_to_http1(
            "Unsupported HTTP method for HTTP/2 replay: invalid method"
        ));
    }

    #[test]
    fn downgrades_http2_request_line_to_http11_for_raw_fallback() {
        let request =
            parse_raw_request("POST /api HTTP/2\r\nHost: example.com\r\n\r\nx=1").unwrap();
        let downgraded = downgrade_request_to_http1(&request);
        let target_url = build_request_url(&request.target, "example.com", 443, true).unwrap();
        let outbound = build_outbound_request(&downgraded, &target_url, &[]);

        assert!(outbound.starts_with("POST /api HTTP/1.1\r\n"));
    }

    #[test]
    fn transport_context_uses_host_header_for_authority_and_sni_override() {
        let request = parse_raw_request("GET /v1 HTTP/2\r\nHost: api.example.com\r\n\r\n").unwrap();
        let target_url = build_request_url(&request.target, "1.2.3.4", 443, true).unwrap();
        let transport = build_replay_transport_context(
            &request,
            &target_url,
            &super::ReplayEndpointInput {
                scheme: "https".to_string(),
                host: "1.2.3.4".to_string(),
                port: 443,
                sni_host: Some("console.volcengine.com".to_string()),
            },
        )
        .unwrap();

        assert_eq!(transport.connect_host, "1.2.3.4");
        assert_eq!(transport.request_authority, "api.example.com");
        assert_eq!(transport.tls_server_name, "console.volcengine.com");
    }

    #[test]
    fn http2_uri_uses_effective_authority_instead_of_connect_host() {
        let request =
            parse_raw_request("GET /v1/list?q=1 HTTP/2\r\nHost: api.example.com\r\n\r\n").unwrap();
        let target_url = build_request_url(&request.target, "1.2.3.4", 443, true).unwrap();
        let transport = build_replay_transport_context(
            &request,
            &target_url,
            &super::ReplayEndpointInput {
                scheme: "https".to_string(),
                host: "1.2.3.4".to_string(),
                port: 443,
                sni_host: None,
            },
        )
        .unwrap();
        let uri = build_http2_request_uri(&request, &target_url, &transport).unwrap();

        assert_eq!(uri.to_string(), "https://api.example.com/v1/list?q=1");
    }

    #[test]
    fn effective_authority_falls_back_to_target_url_when_host_header_absent() {
        let request = parse_raw_request("GET /v1 HTTP/2\r\nAccept: */*\r\n\r\n").unwrap();
        let target_url = Url::parse("https://console.volcengine.com:8443/v1").unwrap();

        let authority = build_effective_request_authority(&request, &target_url).unwrap();

        assert_eq!(authority, "console.volcengine.com:8443");
    }

    #[test]
    fn parses_structured_response_details_from_raw_response() {
        let parsed = parse_raw_response(
            "HTTP/2 302 Found\r\nLocation: /next\r\nSet-Cookie: sid=1\r\nContent-Type: text/plain\r\n\r\nhello",
        )
        .unwrap();

        assert_eq!(parsed.protocol.as_deref(), Some("HTTP/2"));
        assert_eq!(parsed.status_code, 302);
        assert_eq!(parsed.status_text, "Found");
        assert_eq!(parsed.location.as_deref(), Some("/next"));
        assert_eq!(parsed.set_cookie_headers, vec!["sid=1".to_string()]);
        assert_eq!(parsed.body_text, "hello");
        assert_eq!(
            parsed.headers,
            vec![
                ("Location".to_string(), "/next".to_string()),
                ("Set-Cookie".to_string(), "sid=1".to_string()),
                ("Content-Type".to_string(), "text/plain".to_string()),
            ]
        );
    }

    #[test]
    fn display_response_decodes_gzip_body() {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(br#"{"ok":true}"#).unwrap();
        let compressed = encoder.finish().unwrap();

        let raw_response = build_display_response(
            "HTTP/2 200 OK\r\nContent-Type: application/json\r\nContent-Encoding: gzip\r\n",
            &compressed,
            false,
        );
        let parsed = parse_raw_response(&raw_response).unwrap();

        assert_eq!(parsed.body_text, r#"{"ok":true}"#);
        assert!(raw_response.contains("\r\n\r\n{\"ok\":true}"));
    }

    #[test]
    fn display_response_decodes_chunked_gzip_body() {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"hello gzip").unwrap();
        let compressed = encoder.finish().unwrap();
        let chunked = format!("{:X}\r\n", compressed.len()).into_bytes();
        let mut body = chunked;
        body.extend_from_slice(&compressed);
        body.extend_from_slice(b"\r\n0\r\n\r\n");

        let raw_response = build_display_response(
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Encoding: gzip",
            &body,
            true,
        );
        let parsed = parse_raw_response(&raw_response).unwrap();

        assert_eq!(parsed.body_text, "hello gzip");
    }

    #[test]
    fn display_response_decodes_brotli_body() {
        let payload = b"hello brotli";
        let mut compressed = Vec::new();
        {
            let mut writer = brotli::CompressorWriter::new(&mut compressed, 4096, 5, 22);
            writer.write_all(payload).unwrap();
        }

        let raw_response = build_display_response(
            "HTTP/2 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Encoding: br\r\n",
            &compressed,
            false,
        );
        let parsed = parse_raw_response(&raw_response).unwrap();

        assert_eq!(parsed.body_text, "hello brotli");
    }

    #[test]
    fn display_response_decodes_non_utf8_charset_body() {
        let body = [0xC4, 0xE3, 0xBA, 0xC3];

        let raw_response = build_display_response(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=gbk\r\n",
            &body,
            false,
        );
        let parsed = parse_raw_response(&raw_response).unwrap();

        assert_eq!(parsed.body_text, "你好");
    }
}
