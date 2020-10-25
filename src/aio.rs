use crate::{CmsgIter, Frame, Timestamping};
use std::ffi::CStr;
use std::io::{ErrorKind, Result};
use std::os::unix::io::{AsRawFd, RawFd};
use tokio::io::unix::AsyncFd;

pub struct Socket(AsyncFd<crate::Socket>);

impl Socket {
    pub fn bind<I>(ifname: I) -> Result<Self>
    where
        I: AsRef<CStr>,
    {
        let socket = crate::Socket::bind(ifname)?;
        socket.set_nonblocking(true)?;
        Ok(Self(AsyncFd::new(socket)?))
    }

    pub fn set_timestamping(&self, timestamping: Timestamping) -> Result<()> {
        self.0.get_ref().set_timestamping(timestamping)
    }

    pub fn set_recv_own_msgs(&self, enable: bool) -> Result<()> {
        self.0.get_ref().set_recv_own_msgs(enable)
    }

    pub fn set_fd_frames(&self, enable: bool) -> Result<()> {
        self.0.get_ref().set_fd_frames(enable)
    }

    pub async fn recv(&mut self) -> Result<Frame> {
        loop {
            match self.0.readable().await?.with_io(|| self.0.get_ref().recv()) {
                Err(e) if e.kind() == ErrorKind::WouldBlock => (),
                r => break r,
            }
        }
    }

    #[allow(clippy::needless_lifetimes)]
    pub async fn recv_msg<'a>(
        &mut self,
        cmsg_buf: &'a mut [u8],
    ) -> Result<(Frame, Option<CmsgIter<'a>>)> {
        let mut cmsg_buf = Some(cmsg_buf);
        loop {
            let mut guard = self.0.readable().await?;
            match self.0.get_ref()._recv_msg(cmsg_buf.take().unwrap()) {
                Err((e, b)) if e.kind() == ErrorKind::WouldBlock => {
                    cmsg_buf = Some(b);
                    guard.clear_ready();
                }
                r => break r.map_err(|(e, _)| e),
            }
        }
    }

    pub async fn send(&mut self, frame: &Frame) -> Result<()> {
        loop {
            match self
                .0
                .writable()
                .await?
                .with_io(|| self.0.get_ref().send(frame))
            {
                Err(e) if e.kind() == ErrorKind::WouldBlock => (),
                r => break r,
            }
        }
    }
}

impl AsRawFd for Socket {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

#[cfg(test)]
mod tests;
