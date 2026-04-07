use log::{debug, error, warn};
use openssl::ssl::{SslAcceptor, SslConnector, SslMethod, SslVerifyMode};
use std::io::{self, Read, Write};
use std::net::UdpSocket;
use std::sync::Arc;
use std::time::Duration;

/// Wrapper around a connected UdpSocket that implements Read + Write,
/// enabling it to be used with OpenSSL's SslStream for DTLS.
#[derive(Debug)]
struct UdpStream {
    socket: UdpSocket,
}

impl UdpStream {
    fn new(socket: UdpSocket) -> Self {
        UdpStream { socket }
    }
}

impl Read for UdpStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.socket.recv(buf)
    }
}

impl Write for UdpStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.socket.send(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// DTLS-PSK server acceptor configuration.
///
/// Uses Pre-Shared Keys (PSK) for mutual authentication without certificates.
/// The device_secret serves as the PSK, providing both encryption and authentication.
pub struct DtlsServerConfig {
    acceptor: Arc<SslAcceptor>,
}

impl DtlsServerConfig {
    /// Create a new DTLS server config with PSK authentication.
    pub fn new(device_secret: &'static str) -> Result<Self, openssl::error::ErrorStack> {
        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::dtls())?;
        builder.set_verify(SslVerifyMode::NONE);

        // PSK server callback: client sends identity, we return the shared key
        builder.set_psk_server_callback(move |_ssl, identity, psk_out| {
            let identity_str = identity
                .and_then(|id| std::str::from_utf8(id).ok())
                .unwrap_or("");

            debug!("DTLS PSK server callback, identity: {}", identity_str);

            let key = device_secret.as_bytes();
            if key.len() <= psk_out.len() {
                psk_out[..key.len()].copy_from_slice(key);
                Ok(key.len())
            } else {
                warn!("DTLS PSK key too long for buffer");
                Ok(0)
            }
        });

        builder.set_cipher_list("PSK-AES256-CBC-SHA:PSK-AES128-CBC-SHA")?;

        let acceptor = Arc::new(builder.build());
        Ok(DtlsServerConfig { acceptor })
    }

    pub fn acceptor(&self) -> &SslAcceptor {
        &self.acceptor
    }
}

/// DTLS-PSK client connector for sending encrypted CoAP messages to devices.
pub struct DtlsClientConfig {
    device_secret: String,
}

impl DtlsClientConfig {
    pub fn new(device_secret: &str) -> Self {
        DtlsClientConfig {
            device_secret: device_secret.to_string(),
        }
    }

    /// Send a DTLS-encrypted UDP datagram and receive the response.
    ///
    /// One-shot: connect, handshake, send, receive, disconnect.
    /// Suitable for CoAP's request-response pattern.
    pub fn send_receive(
        &self,
        addr: &str,
        data: &[u8],
        timeout: Duration,
    ) -> io::Result<Vec<u8>> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_read_timeout(Some(timeout))?;
        socket.set_write_timeout(Some(timeout))?;
        socket.connect(addr)?;

        let stream = UdpStream::new(socket);
        let device_secret = self.device_secret.clone();

        let mut connector = match SslConnector::builder(SslMethod::dtls()) {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to create DTLS connector: {:?}", e);
                return Err(io::Error::new(io::ErrorKind::Other, e.to_string()));
            }
        };

        connector.set_verify(SslVerifyMode::NONE);

        connector.set_psk_client_callback(move |_ssl, _hint, identity_out, psk_out| {
            let identity = b"homesoil-device";
            if identity.len() < identity_out.len() {
                identity_out[..identity.len()].copy_from_slice(identity);
                identity_out[identity.len()] = 0;
            }

            let key = device_secret.as_bytes();
            if key.len() <= psk_out.len() {
                psk_out[..key.len()].copy_from_slice(key);
                Ok(key.len())
            } else {
                Ok(0)
            }
        });

        connector
            .set_cipher_list("PSK-AES256-CBC-SHA:PSK-AES128-CBC-SHA")
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let connector = connector.build();

        let mut ssl_stream = match connector.connect("", stream) {
            Ok(s) => s,
            Err(e) => {
                error!("DTLS handshake failed with {}: {:?}", addr, e);
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionRefused,
                    format!("{:?}", e),
                ));
            }
        };

        ssl_stream.write_all(data)?;

        let mut buf = vec![0u8; 4096];
        match ssl_stream.read(&mut buf) {
            Ok(n) => {
                buf.truncate(n);
                Ok(buf)
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "DTLS read timed out",
            )),
            Err(e) => Err(e),
        }
    }
}

/// Check if DTLS is required (env var REQUIRE_DTLS=true).
pub fn is_dtls_required() -> bool {
    std::env::var("REQUIRE_DTLS")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
}

/// Get the DTLS port from env or default to 5684.
pub fn get_dtls_port() -> u16 {
    std::env::var("COAP_DTLS_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5684)
}
