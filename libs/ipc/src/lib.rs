#[cfg(feature = "bincode")]
use bincode;
pub use log;

pub mod settings;
pub use settings::*;
#[cfg(not(feature = "bincode"))]
use {
    log::{error, info, trace},
    serde::{Deserialize, Serialize},
    std::{
        fmt::Display,
        io::{BufReader, Read, Write},
        marker::PhantomData,
        os::unix::net::{UnixListener, UnixStream},
        path::PathBuf,
        sync::mpsc::{self, Sender},
    },
};

#[doc(hidden)]
pub use crate as ipsea;

const MAX_FRAME_SIZE: usize = 8 * 1024 * 1024;

fn write_frame<W: Write>(writer: &mut W, bytes: &[u8]) -> std::io::Result<()> {
    if bytes.len() > MAX_FRAME_SIZE {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("frame too large: {}", bytes.len())));
    }
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(bytes)?;
    writer.flush()
}

fn read_frame<R: Read>(reader: &mut R) -> std::io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("frame too large: {}", len)));
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

fn serialize_to_vec<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    #[cfg(feature = "bincode")]
    {
        bincode::serialize(value).map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "bincode"))]
    {
        serde_json::to_vec(value).map_err(|e| e.to_string())
    }
}

fn deserialize_from_slice<'de, T>(bytes: &'de [u8]) -> Result<T, String>
where
    T: Deserialize<'de>,
{
    #[cfg(feature = "bincode")]
    {
        bincode::deserialize(bytes).map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "bincode"))]
    {
        serde_json::from_slice(bytes).map_err(|e| e.to_string())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum StreamResponse<T> {
    Data(T),
    EndOfStream,
}

pub(crate) fn process<Res, Req>(mut stream: UnixStream) -> Option<(Req, Sender<Res>)>
where
    Req: for<'de> Deserialize<'de> + Send + 'static + std::fmt::Debug,
    Res: Serialize + Send + 'static + std::fmt::Debug,
{
    let buf = match read_frame(&mut stream) {
        Ok(b) => {
            trace!("Request bytes: {}", b.len());
            b
        }
        Err(e) => {
            error!("Failed to read request: {}", e);
            return None;
        }
    };

    if let Ok(req) = deserialize_from_slice::<Req>(&buf) {
        info!("Received request: {:?}", req);
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            for response in rx {
                trace!("Sending response: {:?}", response);
                match serialize_to_vec(&StreamResponse::Data(response)) {
                    Ok(resp_buf) => {
                        if let Err(e) = write_frame(&mut stream, &resp_buf) {
                            error!("Failed to send response: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to serialize response: {}", e);
                        break;
                    }
                }
            }
            if let Ok(end_buf) = serialize_to_vec(&StreamResponse::<Res>::EndOfStream) {
                let _ = write_frame(&mut stream, &end_buf);
                info!("Stream ended successfully");
            }
        });

        Some((req, tx))
    } else {
        error!("Failed to deserialize request");
        None
    }
}

pub fn resolve_socket_path(socket_path: impl Into<PathBuf> + Display) -> PathBuf {
    let s = socket_path.to_string();
    if s.starts_with('/') {
        PathBuf::from(s)
    } else if s.ends_with(".sock") {
        PathBuf::from(format!("/tmp/{}", s))
    } else {
        PathBuf::from(format!("/tmp/{}.sock", s))
    }
}

/// Spawns a server that listens for requests, then
/// spawns new (std) threads to handle them.
pub fn start_server<Req, Res, F>(socket_path: impl Into<PathBuf> + Display, handler: F) -> std::io::Result<()>
where
    Req: for<'de> Deserialize<'de> + Send + 'static + std::fmt::Debug,
    Res: Serialize + Send + 'static + std::fmt::Debug,
    F: Fn(Req, Sender<Res>) + Send + Sync + Clone + 'static,
{
    let socket_path = resolve_socket_path(socket_path);

    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;

    info!("Server started on {:?}", socket_path);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                info!("Accepted connection");
                let handler = handler.clone();
                std::thread::spawn(move || {
                    if let Some((req, tx)) = process(stream) {
                        handler(req, tx);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
                continue;
            }
        }
    }
    Ok(())
}

pub struct RequestStream<Req, Res> {
    unix_stream: tokio::net::UnixListener,
    _req: PhantomData<Req>,
    _res: PhantomData<Res>,
}

impl<Req, Res> RequestStream<Req, Res>
where
    Req: for<'de> Deserialize<'de> + Send + 'static + std::fmt::Debug,
    Res: Serialize + Send + 'static + std::fmt::Debug,
{
    pub async fn new(app: impl ToString) -> std::io::Result<Self> {
        let socket_path = resolve_socket_path(app.to_string());
        let _ = std::fs::remove_file(&socket_path);
        let listener = tokio::net::UnixListener::bind(&socket_path)?;
        info!("Server started on {:?}", socket_path);

        Ok(Self { unix_stream: listener, _req: PhantomData, _res: PhantomData })
    }
}

impl<Req, Res> futures::Stream for RequestStream<Req, Res>
where
    Req: for<'de> Deserialize<'de> + Send + 'static + std::fmt::Debug,
    Res: Serialize + Send + 'static + std::fmt::Debug,
{
    type Item = std::io::Result<(Req, Sender<Res>)>;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        match self.unix_stream.poll_accept(cx) {
            std::task::Poll::Ready(Ok((stream, _))) => {
                let std_stream = match stream.into_std() {
                    Ok(v) => v,
                    Err(e) => return std::task::Poll::Ready(Some(Err(e))),
                };

                std::task::Poll::Ready(Some(match process(std_stream) {
                    Some((req, tx)) => Ok((req, tx)),
                    None => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Handshake error")),
                }))
            }
            std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Some(Err(e))),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

/// Spawns a server that delivers requests as a stream.
/// Good for use in select!{} or alike.
pub async fn start_stream<Req, Res>(socket_name: impl ToString) -> std::io::Result<RequestStream<Req, Res>>
where
    Req: for<'de> Deserialize<'de> + Send + 'static + std::fmt::Debug,
    Res: Serialize + Send + 'static + std::fmt::Debug,
{
    RequestStream::new(socket_name).await
}

pub fn send_command<Req, Res, H>(
    socket_path: impl Into<PathBuf> + Display,
    command: &Req,
    handler: Option<H>,
) -> std::io::Result<()>
where
    Req: Serialize,
    Res: for<'de> Deserialize<'de> + std::fmt::Debug, // Debug logging
    H: Fn(Res) + Send + 'static,
{
    let socket_path = resolve_socket_path(socket_path);

    info!("Connecting to server at {:?}", socket_path);
    let mut stream = UnixStream::connect(&socket_path)?;
    let data = serialize_to_vec(command).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    write_frame(&mut stream, &data)?;
    info!("Command sent");

    let mut reader = BufReader::new(stream);

    loop {
        let buf = match read_frame(&mut reader) {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to read response: {}", e);
                break;
            }
        };

        // Deserialize response
        match deserialize_from_slice::<StreamResponse<Res>>(&buf) {
            Ok(StreamResponse::Data(response)) => {
                info!("Received response: {:?}", response);
                if let Some(ref handler) = handler {
                    handler(response);
                }
            }
            Ok(StreamResponse::EndOfStream) => {
                info!("End of stream received");
                break;
            }
            Err(e) => {
                error!("Failed to deserialize response: {}", e);
                break;
            }
        }
    }
    Ok(())
}
pub mod notifications;
pub use notifications::{Notification, NotificationBroadcaster, NotificationEvent, NotificationRequest};
pub mod clipboard;
pub mod modals;
