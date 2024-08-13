use crate::{
    error::Error,
    socket::{Socket, SocketFamily, SocketOption, SocketProtocol, SocketType, SplitSocketHandle},
    tls::PeerVerification,
    CancellationToken, LteLink,
};

use no_std_net::ToSocketAddrs;

/// A TLS (TCP) stream that is connected to another endpoint
pub struct TlsStream {
    inner: Socket,
}

macro_rules! impl_receive {
    () => {
        /// Try fill the given buffer with the data that has been received. The written part of the
        /// buffer is returned.
        pub async fn receive<'buf>(&self, buf: &'buf mut [u8]) -> Result<&'buf mut [u8], Error> {
            self.receive_with_cancellation(buf, &Default::default())
                .await
        }

        /// Try fill the given buffer with the data that has been received. The written part of the
        /// buffer is returned.
        pub async fn receive_with_cancellation<'buf>(
            &self,
            buf: &'buf mut [u8],
            token: &CancellationToken,
        ) -> Result<&'buf mut [u8], Error> {
            let max_receive_len = 1024.min(buf.len());
            let received_bytes = self
                .socket()
                .receive(&mut buf[..max_receive_len], token)
                .await?;
            Ok(&mut buf[..received_bytes])
        }

        /// Fill the entire buffer with data that has been received. This will wait as long as necessary to fill up the
        /// buffer.
        ///
        /// If there's an error while receiving, then the error is returned as well as the part of the buffer that was
        /// partially filled with received data.
        pub async fn receive_exact<'buf>(
            &self,
            buf: &'buf mut [u8],
        ) -> Result<(), (Error, &'buf mut [u8])> {
            self.receive_exact_with_cancellation(buf, &Default::default())
                .await
        }

        /// Fill the entire buffer with data that has been received. This will wait as long as necessary to fill up the
        /// buffer.
        ///
        /// If there's an error while receiving, then the error is returned as well as the part of the buffer that was
        /// partially filled with received data.
        pub async fn receive_exact_with_cancellation<'buf>(
            &self,
            buf: &'buf mut [u8],
            token: &CancellationToken,
        ) -> Result<(), (Error, &'buf mut [u8])> {
            let mut received_bytes = 0;

            while received_bytes < buf.len() {
                match self
                    .receive_with_cancellation(&mut buf[received_bytes..], token)
                    .await
                {
                    Ok(received_data) => received_bytes += received_data.len(),
                    Err(e) => return Err((e.into(), &mut buf[..received_bytes])),
                }
            }

            Ok(())
        }
    };
}

macro_rules! impl_write {
    () => {
        /// Write the entire buffer to the stream
        pub async fn write(&self, buf: &[u8]) -> Result<(), Error> {
            self.write_with_cancellation(buf, &Default::default()).await
        }

        /// Write the entire buffer to the stream
        pub async fn write_with_cancellation(
            &self,
            buf: &[u8],
            token: &CancellationToken,
        ) -> Result<(), Error> {
            let mut written_bytes = 0;

            while written_bytes < buf.len() {
                // We can't write very huge chunks because then the socket can't process it all at once
                let max_write_len = 1024.min(buf.len() - written_bytes);
                written_bytes += self
                    .socket()
                    .write(&buf[written_bytes..][..max_write_len], token)
                    .await?;
            }

            Ok(())
        }
    };
}

impl TlsStream {
    /// Connect a TLS (TCP) stream to the given address
    pub async fn connect(
        addr: impl ToSocketAddrs,
        peer_verify: PeerVerification,
        security_tags: &[u32],
    ) -> Result<Self, Error> {
        Self::connect_with_cancellation(
            addr,
            peer_verify,
            security_tags,
            &Default::default(),
        )
        .await
    }

    /// Connect a TLS stream to the given address
    pub async fn connect_with_cancellation(
        addr: impl ToSocketAddrs,
        peer_verify: PeerVerification,
        security_tags: &[u32],
        token: &CancellationToken,
    ) -> Result<Self, Error> {
        let mut last_error = None;
        let lte_link = LteLink::new().await?;
        let addrs = addr.to_socket_addrs().unwrap();

        for addr in addrs {
            token.as_result()?;

            let family = match addr {
                no_std_net::SocketAddr::V4(_) => SocketFamily::Ipv4,
                no_std_net::SocketAddr::V6(_) => SocketFamily::Ipv6,
            };

            let socket = Socket::create(family, SocketType::Stream, SocketProtocol::Tls1v2).await?;
            socket.set_option(SocketOption::TlsPeerVerify(peer_verify.as_integer()))?;
            socket.set_option(SocketOption::TlsSessionCache(0))?;
            socket.set_option(SocketOption::TlsTagList(security_tags))?;

            match unsafe { socket.connect(addr, token).await } {
                Ok(_) => {
                    lte_link.deactivate().await?;
                    return Ok(TlsStream { inner: socket });
                }
                Err(e) => {
                    last_error = Some(e);
                    socket.deactivate().await?;
                }
            }
        }

        lte_link.deactivate().await?;
        Err(last_error.take().unwrap())
    }

    /// Get the raw underlying file descriptor for when you need to interact with the nrf libraries directly
    pub fn as_raw_fd(&self) -> i32 {
        self.inner.as_raw_fd()
    }

    fn socket(&self) -> &Socket {
        &self.inner
    }

    /// Split the stream into an owned read and write half
    pub async fn split_owned(self) -> Result<(OwnedTlsReadStream, OwnedTlsWriteStream), Error> {
        let (read_split, write_split) = self.inner.split().await?;

        Ok((
            OwnedTlsReadStream { stream: read_split },
            OwnedTlsWriteStream {
                stream: write_split,
            },
        ))
    }

    /// Split the stream into a borrowed read and write half
    pub fn split(&self) -> (TlsReadStream<'_>, TlsWriteStream<'_>) {
        (
            TlsReadStream { stream: self },
            TlsWriteStream { stream: self },
        )
    }

    impl_receive!();
    impl_write!();

    /// Deactivates the socket and the LTE link.
    /// A normal drop will do the same thing, but blocking.
    pub async fn deactivate(self) -> Result<(), Error> {
        self.inner.deactivate().await?;
        Ok(())
    }
}

/// A borrowed read half of a TCP stream
pub struct TlsReadStream<'a> {
    stream: &'a TlsStream,
}

impl<'a> TlsReadStream<'a> {
    fn socket(&self) -> &Socket {
        &self.stream.inner
    }

    impl_receive!();
}

/// A borrowed write half of a TCP stream
pub struct TlsWriteStream<'a> {
    stream: &'a TlsStream,
}

impl<'a> TlsWriteStream<'a> {
    fn socket(&self) -> &Socket {
        &self.stream.inner
    }

    impl_write!();
}

/// An owned read half of a TCP stream
pub struct OwnedTlsReadStream {
    stream: SplitSocketHandle,
}

impl OwnedTlsReadStream {
    fn socket(&self) -> &Socket {
        &self.stream
    }

    impl_receive!();

    /// Deactivates the socket and the LTE link.
    /// A normal drop will do the same thing, but blocking.
    pub async fn deactivate(self) -> Result<(), Error> {
        self.stream.deactivate().await?;
        Ok(())
    }
}

/// An owned write half of a TCP stream
pub struct OwnedTlsWriteStream {
    stream: SplitSocketHandle,
}

impl OwnedTlsWriteStream {
    fn socket(&self) -> &Socket {
        &self.stream
    }

    impl_write!();

    /// Deactivates the socket and the LTE link.
    /// A normal drop will do the same thing, but blocking.
    pub async fn deactivate(self) -> Result<(), Error> {
        self.stream.deactivate().await?;
        Ok(())
    }
}
