use serial::windows::COMPort;
use std::{io::{Read, Write}, sync::Arc, time::{Duration, SystemTime}};
use tokio::{sync::Mutex, time};

//
// this took way to long time, fuck divoom
//
// actually useful stuff, not you divoom
// https://github.com/RomRider/node-divoom-timebox-evo/blob/master/PROTOCOL.md
// https://github.com/d03n3rfr1tz3/hass-divoom/blob/main/custom_components/divoom/devices/divoom.py
//

#[tokio::main]
async fn main() {
    let port: COMPort = serial::open("COM4").unwrap();

    //take some time to connect, mimimimimi
    time::sleep(Duration::from_secs(6)).await;

    let port_locked = Arc::new(Mutex::new(port));
    
    // reader
    let read_port = port_locked.clone();
    tokio::spawn(async move {
        let mut last = SystemTime::now();
        loop {
            let mut read = read_port.lock().await;

            let mut buffer = vec![0; 1024];
            let trying = read.read(&mut buffer);
            match trying {
                Ok(len) => {
                    buffer.truncate(len);

                    println!("{:X?}", buffer);
                }
                Err(err) => {
                    println!("error: {}", err);
                }
            }

            let now = SystemTime::now();
            let diff = now.duration_since(last).unwrap();
            last = now;

            //usually around 100-110ms
            println!("{}ms", diff.as_millis());
        }
    });

    let mut upwards = false;
    let mut value: u8 = 50;

    // update loop
    loop {
        match upwards {
            true => value += 10,
            false => value -= 10,
        }

        if value >= 100 {
            value = 100;
            upwards = false;
        }

        if value == 0 {
            upwards = true;
        }

        send_command(&port_locked, 0x74, vec![value as u8]).await;

        time::sleep(Duration::from_millis(150)).await;
    }
}

async fn send_command(port: &Arc<Mutex<COMPort>>, command: u8, args: Vec<u8>) {
    let length = args.len() + 3;

    let mut payload = vec![];
    payload.extend(littlehex(length as u16));
    payload.push(command);
    payload.extend(args);

    let formatted = format_payload(payload);

    //println!("{:X?}", formatted);
    
    port.lock().await.write(&formatted).unwrap();
}

fn format_payload(payload: Vec<u8>) -> Vec<u8> {
    let mut formatted: Vec<u8> = vec![0x01];

    formatted.extend(&payload);
    formatted.extend(checksum(payload));

    formatted.append(&mut vec![0x02]);
    return formatted;
}

fn checksum(payload: Vec<u8>) -> Vec<u8> {
    let length: u16 = payload.iter().fold(0u16, |sum, i| sum + *i as u16);
    return littlehex(length);
}

fn littlehex(hex: u16) -> Vec<u8> {
    return vec![hex as u8, (hex >> 8) as u8];
}
