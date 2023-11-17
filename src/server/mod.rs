use std::{
    net::{TcpListener, TcpStream},
    num::NonZeroUsize,
    thread,
};

use crossbeam_channel::{Receiver, Sender};
use lru::LruCache;
use protobuf::{CodedInputStream, Message};

use crate::protos::generated::applesauce;

enum Event {
    Disconnect(TcpStream),
    Connect(TcpStream),
    Message(applesauce::Wrapper),
}

pub(crate) fn serve(
    listen_addr: &str,
    connect_addr: &str,
) -> (Sender<applesauce::Wrapper>, Receiver<applesauce::Wrapper>) {
    let connect_addr = connect_addr.to_string();
    let listener = TcpListener::bind(listen_addr).unwrap();

    let (tx, rx) = crossbeam_channel::bounded::<Event>(10);
    let (tx_input, rx_input) = crossbeam_channel::bounded::<applesauce::Wrapper>(10);
    let (tx_output, rx_output) = crossbeam_channel::bounded::<applesauce::Wrapper>(10);

    let tx2 = tx.clone();
    let rx2 = rx.clone();

    {
        // Connect to remote server
        let events_tx = tx.clone();
        let stream = TcpStream::connect(connect_addr).unwrap();
        thread::spawn(move || handle_connection(stream, events_tx));
    }

    thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = stream.unwrap();

            let events_tx = tx.clone();

            thread::spawn(move || handle_connection(stream, events_tx));
        }
    });

    thread::spawn(move || {
        let mut streams: Vec<TcpStream> = vec![];
        let mut proxied_events = LruCache::new(NonZeroUsize::new(100).unwrap());

        for event in rx2.iter() {
            match event {
                Event::Disconnect(stream) => {
                    streams.retain_mut(|s| s.peer_addr().unwrap() != stream.peer_addr().unwrap());
                }
                Event::Connect(stream) => {
                    streams.push(stream);
                }
                Event::Message(wrapper) => {
                    if proxied_events.contains(&wrapper.id) {
                        continue;
                    }
                    proxied_events.put(wrapper.id.clone(), true);

                    tx_output.send(wrapper.clone()).unwrap();
                    for mut stream in streams.iter() {
                        wrapper
                            .write_length_delimited_to_writer(&mut stream)
                            .unwrap();
                    }
                }
            }
        }
    });

    thread::spawn(move || {
        for input in rx_input.iter() {
            tx2.send(Event::Message(input)).unwrap();
        }
    });

    return (tx_input, rx_output);
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

            let wrapper: applesauce::Wrapper = input_stream.read_message().unwrap();
            events_tx.send(Event::Message(wrapper)).unwrap();
        }
    });
}
