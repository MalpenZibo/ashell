//! Common D-Bus utilities and patterns used across services.

#[allow(dead_code)]
use zbus::Connection;

/// Helper for efficient proxy creation with standard configuration.
pub struct ProxyFactory<'a> {
    #[allow(dead_code)]
    conn: &'a Connection,
    #[allow(dead_code)]
    destination: &'static str,
}

impl<'a> ProxyFactory<'a> {
    #[allow(dead_code)]
    pub fn new(conn: &'a Connection, destination: &'static str) -> Self {
        Self { conn, destination }
    }
}
