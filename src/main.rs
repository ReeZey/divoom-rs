mod utils;

use bitvec::array::BitArray;

use serial::windows::COMPort;
use utils::{divoom_command, littlehex, Color, DivoomCommand};
use std::{io::Read, sync::Arc, time::{Duration, SystemTime}};
use tokio::{sync::Mutex, time};
use bitvec::bitvec;
use bitvec::prelude::*;

const DEBUG: bool = false;

#[tokio::main]
async fn main() {
    let port: COMPort = serial::open("COM4").unwrap();

    //take some time to connect, mimimimimi
    time::sleep(Duration::from_secs(6)).await;

    let mut port_locked = Arc::new(Mutex::new(port));

    // reader
    let read_port = port_locked.clone();
    tokio::spawn(async move {
        let mut last = SystemTime::now();
        loop {
            let mut read = read_port.lock().await;

            let mut buffer = vec![0; 1024];
            let result = read.read(&mut buffer);
            match result {
                Ok(len) => {
                    buffer.truncate(len);

                    if DEBUG {
                        println!("{:X?}", buffer);
                    }
                }
                Err(_err) => {
                    //println!("error: {}", err);
                }
            }

            let now = SystemTime::now();
            let diff = now.duration_since(last).unwrap();
            last = now;

            if DEBUG {
                //usually around 100-110ms, this is some sort of polling cause i can still send more messages even if this one always, takes 100ms
                println!("{}ms", diff.as_millis());
            }
        }
    });

    //let mut rand = ThreadRng::default();

    divoom_command(&mut port_locked, DivoomCommand::UpdateBrightness, vec![100]).await;

    let mut color = vec![Color::new(0, 0, 0); 16*16];

    /*
    let img = image::io::Reader::open("image.png").unwrap().decode().unwrap();

    for (x,y, c) in img.pixels() {
        color[(x + y * 16) as usize] = Color::new(c[0], c[1], c[2]);
    }
    */

    // one purple square in each corner
    color[  0] = Color::new(170, 0, 255);
    color[ 15] = Color::new(170, 0, 255);
    color[240] = Color::new(170, 0, 255);
    color[255] = Color::new(170, 0, 255);

    send_image(&mut port_locked, color).await;
}

async fn send_image(port: &mut Arc<Mutex<COMPort>>, pixels: Vec<Color>) {
    let mut color_array: Vec<Color> = vec![];
    let mut pixel_array: Vec<u8> = vec![];
    for pixel_color in pixels {
        /*
        this was a little bit overkill, cause the display was only 16x16

        if color_array.len() > 100 {
            let mut best_diff = 255*3;
            let mut index = 0;

            for (i, c) in color_array.iter().enumerate() {
                let mut diff = 0;
                diff += c.r.abs_diff(pixel_color.r) as i32;
                diff += c.g.abs_diff(pixel_color.g) as i32;
                diff += c.b.abs_diff(pixel_color.b) as i32;

                if diff < best_diff {
                    best_diff = diff;
                    index = i;
                }
            }

            pixel_array.push(index as u8);
        }
        */

        if color_array.contains(&pixel_color) {
            pixel_array.push(color_array.iter().position(|elem| elem == &pixel_color).unwrap() as u8);
            continue;
        }

        pixel_array.push(color_array.len() as u8);
        color_array.push(pixel_color);
    }
    
    if DEBUG {
        println!("------------------------");
        let chunks = pixel_array.chunks(16);
        for chunk in chunks {
            println!("{:?}", chunk);
        }
    }
    

    let mut color_data = vec![];
    for color in &color_array {
        color_data.extend(color.colors());
    };

    if DEBUG {
        println!("{:?}", color_data);
    }

    let nbits = (color_array.len() as f32).log(2.0) / (2 as f32).log(2.0);
    let mut nbits_byte = nbits as u8;

    let diff = nbits - nbits_byte as f32;
    //println!("diff: {}", diff);
    if diff > 0.1 {
        nbits_byte += 1;
    }

    let mut bit_array = bitvec![];
    for byte in pixel_array {
        let bits: BitArray<[u8; 1]> = [byte].into();
        let mut bits_vec = bits.to_bitvec();
        
        //bits_vec.reverse();
        bits_vec.truncate(nbits_byte as usize);
        bit_array.extend(bits_vec);
    }

    let mut pixel_data = vec![];
    for index in 0..(bit_array.len()/8) {
        //println!("{:?}", bit_array[index * 8 .. index * 8 + 8].load_le::<u8>());
        pixel_data.push(bit_array[index * 8 .. index * 8 + 8].load::<u8>());
    }
    
    let mut payload: Vec<u8> = vec![];
    payload.extend([0x00, 0x0A, 0x0A, 0x04]); // idk what this is really, i can change this to anything
    payload.push(0xAA);
    let length = 7 + color_data.len() + pixel_data.len();
    payload.extend(littlehex(length as u16));
    payload.extend([0x00, 0x00, 0x00]);
    payload.push(color_array.len() as u8);
    payload.extend(color_data);
    payload.extend(pixel_data);

    //port.lock().await.write_all(&payload).unwrap();
    
    divoom_command(port, DivoomCommand::UpdateImageFrame, payload).await;
}


