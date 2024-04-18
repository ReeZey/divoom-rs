use std::{io::Write, sync::Arc};

use serial::windows::COMPort;
use tokio::sync::Mutex;

#[allow(dead_code)]
pub(crate) enum DivoomCommand {
    UpdateImageFrame = 0x44,
    UpdateAnimationFrame = 0x49,
    UpdateBrightness = 0x74,
    GetInfo = 0x46
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        return Color { r, g, b }
    }

    pub fn colors(self) -> Vec<u8> {
        return vec![self.r, self.g, self.b];
    }
}

pub async fn divoom_command(port: &mut Arc<Mutex<COMPort>>, command: DivoomCommand, args: Vec<u8>) {
    let length = args.len() + 3;

    let mut payload = vec![];
    payload.extend(littlehex(length as u16));
    payload.push(command as u8);
    payload.extend(args);

    let formatted = format_payload(payload);
    
    port.lock().await.write(&formatted).unwrap();
}

pub fn format_payload(payload: Vec<u8>) -> Vec<u8> {
    let mut formatted: Vec<u8> = vec![0x01];

    formatted.extend(&payload);
    formatted.extend(checksum(payload));

    formatted.append(&mut vec![0x02]);
    return formatted;
}

pub fn checksum(payload: Vec<u8>) -> Vec<u8> {
    let length: u16 = payload.iter().fold(0u16, |sum, i| sum + *i as u16);
    return littlehex(length);
}

pub fn littlehex(hex: u16) -> Vec<u8> {
    return vec![hex as u8, (hex >> 8) as u8];
}