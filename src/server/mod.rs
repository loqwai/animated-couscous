use std::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
};

use protobuf::{CodedInputStream, Message};

use crate::protos::generated::applesauce;

pub(crate) fn serve(listener: TcpListener) {
    let (tx, rx) = mpsc::channel::<TcpStream>();

    thread::spawn(move || {
        for stream in listener.incoming() {
            tx.send(stream.unwrap()).unwrap();
        }
    });

    let mut streams: Vec<TcpStream> = vec![];

    loop {
        for new_stream in rx.try_iter() {
            streams.push(new_stream);
        }

        for mut in_stream in streams.iter() {
            let mut input_stream = CodedInputStream::new(&mut in_stream);
            let input: applesauce::Input = input_stream.read_message().unwrap();

            for mut out_stream in &streams {
                input
                    .write_length_delimited_to_writer(&mut out_stream)
                    .unwrap();
            }
        }
    }
}
