mod utils;

use bitvec::array::BitArray;

use image::{DynamicImage, ImageBuffer, Rgba};
use serial::windows::COMPort;
use utils::{best_color_match, divoom_command, littlehex, Color, DivoomCommand};
use std::{fs::File, io::{BufReader, Read}, sync::Arc, time::{Duration, SystemTime}};
use tokio::sync::Mutex;
use bitvec::bitvec;
use bitvec::prelude::*;

const DEBUG: bool = false;


#[tokio::main]
async fn main() {
    let port: COMPort = COMPort::open("COM3").unwrap();

    tokio::time::sleep(Duration::from_secs(6)).await;

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

    let final_text = "this    text    will    write   itself";

    let mut index = 0;
    let mut forward = true;
    
    //println!("{:?}", buffer);
    loop {
        if index >= final_text.len() {
            forward = false;
        }

        if index == 0 {
            forward = true;
        }
        
        match forward {
            true => index += 1,
            false => index -= 1
        }

        let mut colors = vec![Color::default(); 1024];
        print_text(&final_text.split_at(index).0, &mut colors);

        send_image(&mut port_locked, colors).await;

        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

fn print_text(text: &str, colors: &mut Vec<Color>) {
    let chars = text.chars();

    let mut font_image = image::load(BufReader::new(File::open("font.png").unwrap()), image::ImageFormat::Png).unwrap();
    let mut numbers_image = image::load(BufReader::new(File::open("numbers.png").unwrap()), image::ImageFormat::Png).unwrap();

    for (index, char) in chars.into_iter().enumerate() {
        let x = index as u32 % 8;
        let y = index as u32 / 8;

        if char.is_whitespace() {
            continue;
        }

        if char.is_numeric() {
            add_letter(char as u32, x * 4, y * 6, &mut numbers_image, colors);
        } else {
            add_letter(alpha_to_number(char), x * 4, y * 6, &mut font_image, colors);
        }
    }
}

fn alpha_to_number(character: char) -> u32 {
    return "abcdefghijklnmopqrstuvwxyz".chars().position(|c| c == character.to_ascii_lowercase()).unwrap() as u32;
}

fn add_letter(symbol: u32, place_x: u32, place_y: u32, symbols: &mut DynamicImage, colors: &mut Vec<Color>) {

    let symbol_image = symbols.as_mut_rgba8().unwrap();
    let symbol_buffer = symbol_image.to_vec();

    //println!("{:?}", symbol_buffer);

    let symbol_image_width = symbols.width();

    let start_index = symbol * 3 * 4;

    for y in 0..5 {
        for x in 0..3 {
            let index = (x + y * symbol_image_width) * 4;

            let r = symbol_buffer[(start_index + index + 0) as usize];
            let g = symbol_buffer[(start_index + index + 1) as usize];
            let b = symbol_buffer[(start_index + index + 2) as usize];
            let a = symbol_buffer[(start_index + index + 3) as usize];

            if a == 0 {
                continue;
            }
            
            colors[(place_x + x + (place_y + y) * 32) as usize] = Color::new(r, g, b);
        }
    }
}

async fn send_image(port: &mut Arc<Mutex<COMPort>>, pixels: Vec<Color>) {
    let mut color_array: Vec<Color> = vec![];
    let mut pixel_array: Vec<u8> = vec![];
    for pixel_color in pixels {
        let best = best_color_match(&pixel_color, &color_array);
        if color_array.len() > 8 || best.diff < 128 {
            pixel_array.push(best.index as u8);
            continue;
        }

        if color_array.contains(&pixel_color) {
            pixel_array.push(color_array.iter().position(|elem| elem == &pixel_color).unwrap() as u8);
            continue;
        }

        pixel_array.push(color_array.len() as u8);
        color_array.push(pixel_color);
    }
    
    if DEBUG {
        println!("------------------------");
        let chunks = pixel_array.chunks(32);
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
    payload.extend([0x00, 0x00, 0x03]); // i have no idea what this number actually does but it makes the display go 32x32
    payload.extend(littlehex(color_array.len() as u16));
    payload.extend(color_data);
    payload.extend(pixel_data);

    //port.lock().await.write_all(&payload).unwrap();
    
    divoom_command(port, DivoomCommand::UpdateImageFrame, payload).await;
}


