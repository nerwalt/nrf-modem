//! TLS common module. Code shared between TlsStream and DtlsSocket
//!

#[derive(Debug, Copy, Clone)]
pub enum PeerVerification {
    Enabled,
    Optional,
    Disabled,
}

impl PeerVerification {
    pub fn as_integer(self) -> u32 {
        match self {
            PeerVerification::Enabled => 2,
            PeerVerification::Optional => 1,
            PeerVerification::Disabled => 0,
        }
    }
}
