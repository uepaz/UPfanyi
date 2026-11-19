use crate::model::{AiChannel, AiChannels, AiConfig};
use serde_json::Value;

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

// 单次请求的内存 HTTP 服务；不访问真实厂商、用户配置或磁盘数据库。
pub(crate) struct MockServer {
    pub(crate) endpoint: String,
    pub(crate) requests: Receiver<String>,
}

impl MockServer {
    pub(crate) fn new(status: u16, response: Value) -> Self {
        Self::raw(status, response.to_string())
    }

    pub(crate) fn raw(status: u16, response: String) -> Self {
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
            let http = format!("HTTP/1.1 {status} Mock\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len());
            socket.write_all(http.as_bytes()).unwrap();
        });
        Self { endpoint, requests }
    }

    pub(crate) fn captured(&self) -> String {
        self.requests.recv_timeout(Duration::from_secs(5)).unwrap()
    }
}
