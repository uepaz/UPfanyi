use crate::model::{AiChannel, AiChannels, AiConfig};
use serde_json::{json, Value};

/// 虚构用户渠道集合；测试显式使用新契约，不给正式保存增加旧配置兼容入口。
pub(crate) fn channel_collection(config: &AiConfig) -> AiChannels {
    let mut value = AiChannels::default();
    let id = "11111111-1111-4111-8111-111111111111".to_owned();
    value.channels.push(AiChannel {
        id: id.clone(),
        name: "Fixture service".into(),
        config: config.clone(),
    });
    value.active_id = id;
    value
}
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

pub(crate) fn chat_event(text: &str) -> String {
    let value = json!({"choices":[{"index":0,"delta":{"content":text}}]});
    format!("data: {value}\n\n")
}

// 单次请求的内存 HTTP 服务；不访问真实厂商、用户配置或磁盘数据库。
pub(crate) struct MockServer {
    pub(crate) endpoint: String,
    pub(crate) requests: Receiver<String>,
}

struct StreamTail {
    gate: Receiver<()>,
    chunks: Vec<Vec<u8>>,
}

impl MockServer {
    pub(crate) fn new(status: u16, response: Value) -> Self {
        Self::raw(status, response.to_string())
    }

    pub(crate) fn raw(status: u16, response: String) -> Self {
        Self::start(status, "application/json", response.into_bytes(), None)
    }

    pub(crate) fn events(body: &str) -> Self {
        Self::start(
            200,
            "text/event-stream; charset=utf-8",
            body.as_bytes().to_vec(),
            None,
        )
    }

    pub(crate) fn chat(text: &str) -> Self {
        Self::events(&format!("{}data: [DONE]\n\n", chat_event(text)))
    }

    pub(crate) fn responses(response: Value) -> Self {
        let completed = json!({"type":"response.completed","response":response});
        Self::events(&format!("event: response.completed\ndata: {completed}\n\n"))
    }

    /// 先发送部分字节，剩余部分由测试放行。可在任意 UTF-8 字节或 SSE 帧边界暂停。
    pub(crate) fn split_stream(first: &[u8], last: &[u8]) -> (Self, mpsc::Sender<()>) {
        Self::stream_steps(first, vec![last.to_vec()])
    }

    pub(crate) fn stream_steps(first: &[u8], chunks: Vec<Vec<u8>>) -> (Self, mpsc::Sender<()>) {
        let (release, gate) = mpsc::channel();
        (
            Self::start(
                200,
                "text/event-stream",
                first.to_vec(),
                Some(StreamTail { gate, chunks }),
            ),
            release,
        )
    }

    fn start(
        status: u16,
        content_type: &'static str,
        first: Vec<u8>,
        tail: Option<StreamTail>,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!(
            "http://{}/complete/chat?fixture=yes",
            listener.local_addr().unwrap()
        );
        listener.set_nonblocking(true).unwrap();
        let (sender, requests) = mpsc::channel();
        thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let (mut socket, _) = loop {
                match listener.accept() {
                    Ok(socket) => break socket,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        thread::sleep(Duration::from_millis(5))
                    }
                    _ => return,
                }
            };
            // Windows accept 得到的连接可能继承监听器的非阻塞状态。
            socket.set_nonblocking(false).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            socket
                .set_write_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let count = socket.read(&mut buffer).unwrap();
                if count == 0 {
                    break;
                }
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]);
                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            if name.eq_ignore_ascii_case("content-length") {
                                value.trim().parse::<usize>().ok()
                            } else {
                                None
                            }
                        })
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + content_length {
                        break;
                    }
                }
            }
            let _ = sender.send(String::from_utf8(bytes).unwrap());
            let length = first.len()
                + tail
                    .as_ref()
                    .map_or(0, |tail| tail.chunks.iter().map(Vec::len).sum());
            let http = format!("HTTP/1.1 {status} Mock\r\nContent-Type: {content_type}\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n");
            socket.write_all(http.as_bytes()).unwrap();
            if socket.write_all(&first).is_err() {
                return;
            }
            if let Some(tail) = tail {
                for chunk in tail.chunks {
                    if tail.gate.recv_timeout(Duration::from_secs(5)).is_err()
                        || socket.write_all(&chunk).is_err()
                    {
                        break;
                    }
                }
            }
        });
        Self { endpoint, requests }
    }

    pub(crate) fn captured(&self) -> String {
        self.requests.recv_timeout(Duration::from_secs(5)).unwrap()
    }
}
