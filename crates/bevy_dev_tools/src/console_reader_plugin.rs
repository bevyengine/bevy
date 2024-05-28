/// Contains async console read in world

use std::sync::mpsc::{Receiver, Sender};

pub struct ConsoleReaderPlugin;


fn console_reader(mut sender: Sender<ConsoleInput>) {
    while true {
        let result_input 
    }
}

enum ConsoleInput {
    Text(String),
    Quit,
}
struct ConsoleReader {
    receiver: Receiver<ConsoleInput>,
}

