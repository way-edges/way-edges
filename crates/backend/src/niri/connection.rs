use niri_ipc::{socket::SOCKET_PATH_ENV, Reply, Window, WindowLayout, Workspace};
use serde::Deserialize;
use serde_jsonrc::Value;
use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

#[derive(Deserialize, Debug, Clone)]
pub enum Event {
    WorkspacesChanged {
        workspaces: Vec<Workspace>,
    },
    WorkspaceActivated {
        #[allow(dead_code)]
        id: u64,
        #[allow(dead_code)]
        focused: bool,
    },
    WindowsChanged {
        windows: Vec<Window>,
    },
    WindowOpenedOrChanged {
        window: Window,
    },
    WindowClosed {
        id: u64,
    },
    WindowFocusChanged {
        id: Option<u64>,
    },
    WindowLayoutsChanged {
        changes: Vec<(u64, WindowLayout)>,
        #[serde(flatten)]
        _payload: Option<Value>,
    },
    WindowUrgencyChanged {
        id: u64,
        is_urgent: bool,
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
        buf.push('\n');
        self.0.write_all(buf.as_bytes()).await?;

        buf.clear();
        let mut reader = BufReader::new(&mut self.0);
        reader.read_line(&mut buf).await?;

        Ok(serde_jsonrc::from_str(buf.as_str()).unwrap())
    }
}

pub struct Listener(BufReader<UnixStream>);
impl Listener {
    pub async fn next_event(&mut self, buf: &mut String) -> io::Result<Option<Event>> {
        self.0.read_line(buf).await.map(|_| {
            log::debug!("Received niri event: {buf}");
            serde_jsonrc::from_str(buf).ok()
        })
    }
}
