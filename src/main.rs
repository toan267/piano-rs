use rustbox::{Color, RustBox};
use clap::value_t;
use std::default::Default;
use std::sync::{Arc, Mutex};
use std::{thread, time};
use std::net::SocketAddr;
use std::io::Result;

use piano_rs::arguments;
use piano_rs::game::{
    PianoKeyboard,
    GameEvent,
};
use piano_rs::network::{
    NetworkEvent,
    Receiver,
    Sender,
};

fn main() -> Result<()> {
    let matches = arguments::get_arguments();
    // A workaround to stop cracking noise after note ends (issue #4)
    let blank_point = rodio::default_output_device().unwrap();
    let blank_sink = rodio::Sink::new(&blank_point);
    let blank_source = rodio::source::SineWave::new(0);
    blank_sink.append(blank_source);

    let volume = value_t!(matches.value_of("volume"), f32).unwrap_or(1.0);
    let mark_duration = value_t!(matches.value_of("markduration"), u64).unwrap_or(500);

    /* if let Some(playfile) = matches.value_of("play") { */
    /*     let replaycolor = matches.value_of("replaycolor").unwrap_or("blue"); */
    /*     let tempo = value_t!(matches.value_of("tempo"), f32).unwrap_or(1.0); */
    /*     /1* play::play_from_file(playfile, replaycolor, *1/ */
    /*                          /1* mark_duration, volume, tempo, &rustbox); *1/ */
    /* } */

    let sequence = value_t!(matches.value_of("sequence"), i8).unwrap_or(2);
    let sound_duration = value_t!(matches.value_of("noteduration"), u64).unwrap_or(0);
    /* let record_file = matches.value_of("record"); */

    let bind_interface: &str = "0.0.0.0";

    let receiver_port: u16 = 9999;
    let receiver_addr: SocketAddr = format!("{}:{}", &bind_interface, &receiver_port)
        .parse()
        .unwrap();

    let sender_port: u16 = 9998;
    let sender_addr: SocketAddr = format!("{}:{}", &bind_interface, &sender_port)
        .parse()
        .unwrap();

    let host_addr = value_t!(matches.value_of("host"), SocketAddr).unwrap_or(receiver_addr);
    let event_receiver = Receiver::new(receiver_addr)?;
    let event_sender = Arc::new(Mutex::new(Sender::new(sender_addr, host_addr)?));
    let event_sender_clone = event_sender.clone();

    let rustbox = Arc::new(Mutex::new(
        RustBox::init(Default::default()).unwrap()
    ));

    let keyboard = Arc::new(Mutex::new(PianoKeyboard::new(
        sequence,
        volume,
        time::Duration::from_millis(sound_duration),
        time::Duration::from_millis(mark_duration),
        Color::Blue,
    )));

    keyboard.lock().unwrap().draw(&rustbox);

    let clonebox = rustbox.clone();
    let cloneboard = keyboard.clone();

    thread::spawn(move || {
        loop {
            let data = event_receiver.poll_event().unwrap();
            match data.event {
                NetworkEvent::PlayerJoin => {
                    let remote_receiver_addr: SocketAddr = format!("{}:9999", data.src.ip())
                        .parse()
                        .unwrap();

                    event_sender_clone.lock().unwrap()
                        .register_remote_socket(remote_receiver_addr)
                        .unwrap();
                }
                NetworkEvent::Peers(mut peers) => {
                    peers[0] = format!("{}:9999", data.src.ip()).parse().unwrap();
                    event_sender_clone.lock().unwrap().peer_addrs = peers;
                }
                NetworkEvent::ID(id) => {
                    cloneboard.lock().unwrap().set_note_color(match id {
                        0 => Color::Blue,
                        1 => Color::Red,
                        2 => Color::Green,
                        3 => Color::Yellow,
                        4 => Color::Cyan,
                        5 => Color::Magenta,
                        _ => Color::Black,
                    });
                }
                NetworkEvent::Note(note) => {
                    cloneboard.lock().unwrap().play_note(note, &clonebox);
                }
               _ => { },
            }
        }
    });

    event_sender.lock().unwrap().register_self()?;

    let duration = time::Duration::from_nanos(1000);
    loop {
        let event = rustbox.lock().unwrap().peek_event(duration, false);
        match event {
            Ok(rustbox::Event::KeyEvent(key)) => {
                match keyboard.lock().unwrap().process_key(key) {
                    Some(GameEvent::Note(note)) => {
                        event_sender.lock().unwrap().tick(note).unwrap();
                    }
                    Some(GameEvent::Quit) => break,
                    None => { },
                };
            }
            Err(e) => panic!("{}", e),
            _ => { },
        }
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn check_missing_notes() {
        // Find missing notes in assets/*.ogg, if any
        let mut missing_notes = Vec::new();
        let expected_notes = ["a", "as", "b", "c", "cs", "d", "ds", "e", "f", "fs", "g", "gs"];
        for expected_note in expected_notes.iter() {
            if expected_note == &"a" || expected_note == &"as" {
                let note = format!("{}-1.ogg", expected_note);
				let note_path = format!("assets/{}", note);
                if !Path::new(&note_path).exists() {
                    missing_notes.push(note);
                }
            }
			for sequence in 0..8_u16 {
				let note = format!("{}{}.ogg", expected_note, sequence);
				let note_path = format!("assets/{}", note);
                if !Path::new(&note_path).exists() {
                    missing_notes.push(note);
                }
            }
        }

        assert!(missing_notes.len() == 0,
                "Some note sounds are missing: {}", missing_notes.join(", "));
    }
}
