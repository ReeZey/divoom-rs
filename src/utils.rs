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
    pub r: u8,
    pub g: u8,
    pub b: u8
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        return Color { r, g, b }
    }

    pub fn colors(self) -> Vec<u8> {
        return vec![self.r, self.g, self.b];
    }
}

impl Default for Color {
    fn default() -> Self {
        Self { r: 0, g: 0, b: 0 }
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

    //println!("PAYLOAD LENGTH: {}", payload.len());

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

pub struct BestColor {
    pub index: usize,
    pub diff: u32,
}

impl Default for BestColor {
    fn default() -> Self {
        Self { index: 0, diff: 255*3 }
    }
}

pub fn best_color_match(color: &Color, color_array: &Vec<Color>) -> BestColor {
    let mut best = BestColor::default();

    for (i, c) in color_array.iter().enumerate() {
        let mut diff = 0;
        diff += c.r.abs_diff(color.r) as u32;
        diff += c.g.abs_diff(color.g) as u32;
        diff += c.b.abs_diff(color.b) as u32;

        if diff < best.diff {
            best.diff = diff;
            best.index = i;
        }
    }

    return best;
}