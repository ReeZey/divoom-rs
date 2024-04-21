mod utils;

use bitvec::array::BitArray;

use serial::windows::COMPort;
use utils::{best_color_match, divoom_command, littlehex, Color, DivoomCommand};
use windows_capture::{capture::GraphicsCaptureApiHandler, encoder::ImageEncoder, frame::{Frame, ImageFormat}, graphics_capture_api::InternalCaptureControl, monitor::Monitor, settings::{ColorFormat, CursorCaptureSettings, DrawBorderSettings, Settings}};
use std::{io::{self, Read, Write}, sync::Arc, time::{Duration, SystemTime}};
use tokio::{sync::Mutex, time};
use bitvec::bitvec;
use bitvec::prelude::*;

const DEBUG: bool = false;

struct Capture {
    port: COMPort,
    last: SystemTime
}


impl GraphicsCaptureApiHandler for Capture {
    type Flags = String;

    type Error = Box<dyn std::error::Error + Send + Sync>;

    // Function that will be called to create the struct. The flags can be passed from settings.
    fn new(_message: Self::Flags) -> Result<Self, Self::Error> {
        let port: COMPort = COMPort::open("COM4").unwrap();
        
        futures::executor::block_on(async {
            tokio::time::sleep(Duration::from_secs(6)).await;
        });

        Ok(Self { port, last: SystemTime::now() })
    }

    // Called every time a new frame is available.
    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        io::stdout().flush()?;

        let mut binding = frame.buffer().unwrap();
        let buffer = binding.as_raw_buffer();

        let mut cool_buffer = vec![];

        for y in 0..32 {
            for x in 0..32 {

                let base_x = x * 60;
                let base_y = (y * 33) * 1920;
                let base_index = base_x + base_y;

                let mut total_r: u32 = 0;
                let mut total_g: u32 = 0;
                let mut total_b: u32 = 0;
                for color_x in 0..60 {
                    for color_y in 0..32 {
                        let next_index = (base_index + color_x + (color_y * 1920)) * 4;
                        
                        let r = buffer[next_index + 0];
                        let g = buffer[next_index + 1];
                        let b = buffer[next_index + 2];

                        total_r += r as u32;
                        total_g += g as u32;
                        total_b += b as u32;
                    }
                }

                total_r /= 60;
                total_r /= 32;

                total_g /= 60;
                total_g /= 32;

                total_b /= 60;
                total_b /= 32;

                cool_buffer.push(Color::new(total_r as u8, total_g as u8, total_b as u8))
            }
        }

        /*
        for y in 0..32 {
            for x in 0..32 {
                let index = (((x + 1920 / 2) - 16) + ((y + 1080 / 2) - 16) * 1920) * 4; //+ width_offset + height_offset;

                let r = buffer[index + 0];
                let g = buffer[index + 1];
                let b = buffer[index + 2];

                cool_buffer.push(Color::new(r, g, b))
            }
        }
        */

        futures::executor::block_on(async {
            let now = SystemTime::now();
            if now.duration_since(self.last).unwrap().as_millis() > 20 {
                //println!("image");

                send_image(&mut self.port, cool_buffer).await;

                //TODO: fix
                let mut run = true;
                while run {
                    let mut buffer = vec![0; 1024];
                    let result = self.port.read(&mut buffer);
                    match result {
                        Ok(len) => {
                            buffer.truncate(len);
                        }
                        Err(_err) => {
                            //println!("error: {}", err);
                            run = false;
                        }
                    }
                }

                self.last = SystemTime::now();
            }
        });

        //capture_control.stop();

        Ok(())
    }

    // Optional handler called when the capture item (usually a window) closes.
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        println!("Capture Session Closed");

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    /*
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

    */

    //let mut rand = ThreadRng::default();

    //divoom_command(&mut port_locked, DivoomCommand::UpdateBrightness, vec![100]).await;
    
    let monitor = Monitor::primary().unwrap();

    // Setup settings for capturing video
    let capture_settings = Settings::new(
        monitor,
        windows_capture::settings::CursorCaptureSettings::Default,
        windows_capture::settings::DrawBorderSettings::WithoutBorder,
        windows_capture::settings::ColorFormat::Rgba8,
        "aa".to_string()
    ).unwrap();

    Capture::start(capture_settings).expect("Screen Capture Failed");
}

async fn send_image(port: &mut COMPort, pixels: Vec<Color>) {
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


