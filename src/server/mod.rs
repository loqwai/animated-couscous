use std::{
    net::{TcpListener, TcpStream},
    thread,
};

use crossbeam_channel::Sender;
use protobuf::{CodedInputStream, Message};

use crate::protos::generated::applesauce;

enum Event {
    Disconnect(TcpStream),
    Connect(TcpStream),
    Input(applesauce::Input),
}

pub(crate) fn serve(listener: TcpListener) {
    let (tx, rx) = crossbeam_channel::bounded::<Event>(10);

    let rx2 = rx.clone();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = stream.unwrap();

            let events_tx = tx.clone();

            thread::spawn(move || {
                handle_connection(stream, events_tx);
            });
        }
    });

    thread::spawn(move || {
        let mut streams: Vec<TcpStream> = vec![];

        for event in rx2.iter() {
            match event {
                Event::Disconnect(stream) => {
                    streams.retain_mut(|s| s.peer_addr().unwrap() != stream.peer_addr().unwrap());
                }
                Event::Connect(stream) => {
                    streams.push(stream);
                }
                Event::Input(input) => {
                    for mut stream in streams.iter() {
                        input.write_length_delimited_to_writer(&mut stream).unwrap();
                    }
                }
            }
        }
    });
}

fn handle_connection(stream: TcpStream, events_tx: Sender<Event>) {
    events_tx
        .send(Event::Connect(stream.try_clone().unwrap()))
        .unwrap();

    thread::spawn(move || {
        let mut input_stream = stream.try_clone().unwrap();
        let mut input_stream = CodedInputStream::new(&mut input_stream);
        loop {
            if input_stream.eof().unwrap() {
                events_tx.send(Event::Disconnect(stream)).unwrap();
                return;
            }

            let input: applesauce::Input = input_stream.read_message().unwrap();
            events_tx.send(Event::Input(input)).unwrap();
        }
    });
}
