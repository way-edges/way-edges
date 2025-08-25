use niri_ipc::{socket::SOCKET_PATH_ENV, Reply, Workspace};
use serde::Deserialize;
use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

/// A compositor event.
#[derive(Deserialize, Debug, Clone)]
pub enum Event {
    /// The workspace configuration has changed.
    WorkspacesChanged {
        /// The new workspace configuration.
        ///
        /// This configuration completely replaces the previous configuration. I.e. if any
        /// workspaces are missing from here, then they were deleted.
        workspaces: Vec<Workspace>,
    },
    /// A workspace was activated on an output.
    ///
    /// This doesn't always mean the workspace became focused, just that it's now the active
    /// workspace on its output. All other workspaces on the same output become inactive.
    WorkspaceActivated {
        /// Id of the newly active workspace.
        #[allow(dead_code)]
        id: u64,
        /// Whether this workspace also became focused.
        ///
        /// If `true`, this is now the single focused workspace. All other workspaces are no longer
        /// focused, but they may remain active on their respective outputs.
        #[allow(dead_code)]
        focused: bool,
    },
}

pub struct Connection(UnixStream);
impl Connection {
    pub async fn make_connection() -> io::Result<Connection> {
        let socket_path = std::env::var_os(SOCKET_PATH_ENV).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("{SOCKET_PATH_ENV} is not set, are you running this within niri?"),
            )
        })?;
        let s = UnixStream::connect(socket_path).await?;
        Ok(Self(s))
    }
    #[allow(clippy::wrong_self_convention)]
    pub async fn to_listener(mut self) -> io::Result<Listener> {
        self.push_request(niri_ipc::Request::EventStream)
            .await?
            .expect("Failed to open event stream");

        let reader = BufReader::new(self.0);

        Ok(Listener(reader))
    }
    pub async fn push_request(&mut self, req: niri_ipc::Request) -> io::Result<Reply> {
        let mut buf = serde_jsonrc::to_string(&req).unwrap();
        self.0.write_all(buf.as_bytes()).await?;
        self.0.shutdown().await?;

        buf.clear();
        BufReader::new(&mut self.0).read_line(&mut buf).await?;

        Ok(serde_jsonrc::from_str(buf.as_str()).unwrap())
    }
}

pub struct Listener(BufReader<UnixStream>);
impl Listener {
    pub async fn next_event(&mut self, buf: &mut String) -> io::Result<Option<Event>> {
        self.0.read_line(buf).await.map(|_| {
            serde_jsonrc::from_str(buf)
                .inspect_err(|e| log::warn!("Unhandled niri event: {e}"))
                .ok()
        })
    }
}
