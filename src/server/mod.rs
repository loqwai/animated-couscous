use std::{
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use crossbeam_channel::{Receiver, Sender};
use protobuf::{CodedInputStream, Message};

use crate::protos::generated::applesauce;

enum Event {
    Streams(Vec<TcpStream>),
    Input(applesauce::Input),
}

pub(crate) fn serve(listener: TcpListener) {
    let (tx, rx) = crossbeam_channel::bounded::<Event>(10);

    thread::spawn(move || {
        let mut streams: Vec<TcpStream> = vec![];

        for stream in listener.incoming() {
            let stream = stream.unwrap();
            streams.push(stream.try_clone().unwrap());
            let events_tx = tx.clone();
            let events_rx = rx.clone();

            thread::spawn(move || {
                handle_connection(stream, events_tx, events_rx);
            });

            thread::sleep(Duration::from_millis(10));
            tx.send(Event::Streams(clone_streams(&streams))).unwrap();
        }
    });
}

fn clone_streams(streams: &Vec<TcpStream>) -> Vec<TcpStream> {
    let mut new_streams: Vec<TcpStream> = vec![];

    for stream in streams.iter() {
        new_streams.push(stream.try_clone().unwrap());
    }

    new_streams
}

fn handle_connection(mut stream: TcpStream, events_tx: Sender<Event>, events_rx: Receiver<Event>) {
    thread::spawn(move || loop {
        let mut input_stream = CodedInputStream::new(&mut stream);
        if input_stream.eof().unwrap() {
            return;
        }

        let input: applesauce::Input = input_stream.read_message().unwrap();
        events_tx.send(Event::Input(input)).unwrap();
    });

    let mut streams: Vec<TcpStream> = vec![];
    for event in events_rx.iter() {
        match event {
            Event::Streams(new_streams) => streams = new_streams,
            Event::Input(input) => {
                for mut stream in streams.iter() {
                    input.write_length_delimited_to_writer(&mut stream).unwrap();
                }
            }
        }
    }
}
