mod io;

use std::time::Duration;
use byteorder::WriteBytesExt;
use tokio::net::{TcpListener, TcpStream};
use log::{error, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::{BufMut, BytesMut};
use serde_json::json;
use crate::io::{read_string, read_var_int, size_in_bytes, write_string, write_var_int};

#[tokio::main]
async fn main() {
    let listener = match TcpListener::bind("0.0.0.0:25565").await {
        Ok(listener) => listener,
        Err(e) => {
            error!("Failed to start TCP listener: {e}");
            return;
        }
    };
    println!("Started listening on: {}", listener.local_addr().unwrap());

    let timeout = Duration::from_secs(5);

    // accept connections and process them
    loop {
        match listener.accept().await {
            Ok((stream, address)) => {
                println!("Init connection: {address}");
                tokio::spawn(async move {
                    if let Err(error) = tokio::time::timeout(
                        timeout,
                        handle_client(stream),
                    ).await { warn!("Initial connection timed out: {error}"); }
                });
            }
            Err(error) => {
                error!("Failed to accept connection: {error}");
                return;
            }
        };
    }
}

async fn handle_client(mut stream: TcpStream) {
    if let Err(error) = stream.set_nodelay(true) {
        error!("Failed to set tcp nodelay: {error}");
    }

    let mut vec = vec![0u8; stream.read_i8().await.unwrap() as usize];
    stream.read_exact(&mut vec).await.unwrap();
    let mut buf = &vec[..];
    if buf.read_i8().await.unwrap() != 0 { // Should be Handshake!
        stream.shutdown().await.unwrap();
        return;
    }
    let protocol_version = read_var_int(&mut buf).unwrap();
    let server_address = read_string(&mut buf).unwrap();
    let server_port = buf.read_u16().await.unwrap();
    let state = buf.read_i8().await.unwrap();
    println!("{protocol_version}, {server_address}, {server_port}, {state}");

    if state == 1 {
        let buf = &mut BytesMut::new();
        let mut writer = buf.writer();
        writer.write_u8(0).unwrap();
        write_string(writer, json!({
            "version": {
                "name": "Proxy",
                "protocol": protocol_version
            },
            "players": {
                "online": 0,
                "max": 5
            },
            "description": {
                "text": "Another weird minecraft proxy!",
                "color": "aqua"
            }
        }).to_string());
        write(stream, buf).await;
    } else {
        let buf = &mut BytesMut::new();
        let mut writer = buf.writer();
        writer.write_u8(0).unwrap();
        write_string(writer, String::from("{\"text\":\"Disconnect\",\"color\":\"red\"}"));
        write(stream, buf).await;
    }

    pub async fn write(mut stream: TcpStream, buf: &mut BytesMut) {
        let packet_length = buf.len();
        let packet_length_size = size_in_bytes(packet_length as i32);

        // we need to write the packet length before the packet
        buf.put_bytes(0, packet_length_size);
        buf.copy_within(..packet_length, packet_length_size);

        let buf_front = &mut buf[..];
        write_var_int(buf_front, packet_length as i32);
        stream.write_all(&*buf).await.unwrap();
        stream.flush().await.unwrap();
        stream.shutdown().await.unwrap();
    }
}