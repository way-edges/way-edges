use niri_ipc::{socket::SOCKET_PATH_ENV, Reply};
use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

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
    pub async fn next_event(&mut self, buf: &mut String) -> io::Result<niri_ipc::Event> {
        self.0
            .read_line(buf)
            .await
            .map(|_| serde_jsonrc::from_str(buf).unwrap())
    }
}
