/// Contains async console read in world

use std::sync::{mpsc::{Receiver, Sender}, Arc, Mutex, RwLock};

use bevy_app::{Plugin, PreUpdate};
use bevy_ecs::{event::{Event, EventWriter}, system::{ResMut, Resource}};

pub struct ConsoleReaderPlugin;

impl Plugin for ConsoleReaderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let console_reader = create_console_bridge();

        app.insert_resource(console_reader);
        app.add_event::<ConsoleInput>();

        app.add_systems(PreUpdate, console_reader_system);
    }
}

fn console_reader_system(
    mut console_reader: ResMut<ConsoleReader>,
    mut events: EventWriter<ConsoleInput>,
) {
    while let Ok(input) = console_reader.receiver.lock().unwrap().recv() {
        events.send(input);
    }
}

fn async_console_reader(mut reader: AsyncConsoleReader) {
    let mut editor = rustyline::DefaultEditor::new().unwrap();
    while true {
        let result_input = editor.readline(">> ");

        match result_input {
            Ok(input) => {
                reader.sender.send(ConsoleInput::Text(input)).unwrap();
            }
            Err(_) => {
                reader.sender.send(ConsoleInput::Quit).unwrap();
                break;
            }
        }
    }
}

#[derive(Debug, Event)]
pub enum ConsoleInput {
    Text(String),
    Quit,
}

#[derive(Resource)]
struct ConsoleReader {
    receiver: Arc<Mutex<Receiver<ConsoleInput>>>,
    sender: Arc<Mutex<Sender<ToConsoleEditor>>>,
    async_thread: RwLock<Option<std::thread::JoinHandle<()>>>,
}

enum ToConsoleEditor {
    Interrupt,
}
struct AsyncConsoleReader {
    sender: Sender<ConsoleInput>,
    receiver: Receiver<ToConsoleEditor>,
}

fn create_console_bridge() -> ConsoleReader{
    let (sender, receiver) = std::sync::mpsc::channel();
    let (sender_to_editor, receiver_to_editor) = std::sync::mpsc::channel();

    ConsoleReader {
        receiver: Arc::new(Mutex::new(receiver)),
        sender: Arc::new(Mutex::new(sender_to_editor)),
        async_thread: RwLock::new(Some(
            std::thread::spawn(|| async_console_reader(AsyncConsoleReader {
                sender,
                receiver: receiver_to_editor,
            })),
        )),
    }
}