use undo_2::Action;
use undo_2::Action::*;
use undo_2::Commands;
#[test]
#[allow(unused)]
fn application() {
    enum Command {
        Add(char),
        Delete(char),
    }
    struct TextEditor {
        text: String,
        command: Commands<Command>,
    }
    impl TextEditor {
        pub fn new() -> Self {
            Self {
                text: String::new(),
                command: Commands::new(),
            }
        }
        pub fn add_char(&mut self, c: char) {
            self.text.push(c);
            self.command.push(Command::Add(c));
        }
        pub fn delete_char(&mut self) {
            if let Some(c) = self.text.pop() {
                self.command.push(Command::Delete(c));
            }
        }
        pub fn undo(&mut self) {
            for action in self.command.undo() {
                interpret_action(&mut self.text, action)
            }
        }
        pub fn redo(&mut self) {
            for action in self.command.redo() {
                interpret_action(&mut self.text, action)
            }
        }
    }
    fn interpret_action(data: &mut String, action: Action<&Command>) {
        use Action::*;
        use Command::*;
        match action {
            Do(Add(c)) | Undo(Delete(c)) => {
                data.push(*c);
            }
            Undo(Add(_)) | Do(Delete(_)) => {
                data.pop();
            }
        }
    }
    let mut editor = TextEditor::new();
    editor.add_char('a'); //              :[1]
    editor.add_char('b'); //              :[2]
    editor.add_char('d'); //              :[3]
    assert_eq!(editor.text, "abd");

    editor.undo(); // first undo          :[4]
    assert_eq!(editor.text, "ab");

    editor.add_char('c'); //              :[5]
    assert_eq!(editor.text, "abc");

    editor.undo(); // Undo [5]            :[6]
    assert_eq!(editor.text, "ab");
    editor.undo(); // Undo the undo [4]   :[7]
    assert_eq!(editor.text, "abd");
    editor.undo(); // Undo [3]            :[8]
    assert_eq!(editor.text, "ab");
    editor.undo(); // Undo [2]            :[9]
    assert_eq!(editor.text, "a");

    editor.add_char('z'); //              :[10]
    assert_eq!(editor.text, "az");
    // when an action is performed after a sequence
    // of undo, the undos are merged: undos [6] to [9] are merged now

    editor.undo(); // back to [10]
    assert_eq!(editor.text, "a");
    editor.undo(); // back to [5]: reverses the consecutive sequence of undos in batch
    assert_eq!(editor.text, "abc");
    editor.undo(); // back to [4]
    assert_eq!(editor.text, "ab");
    editor.undo(); // back to [3]
    assert_eq!(editor.text, "abd");
    editor.undo(); // back to [2]
    assert_eq!(editor.text, "ab");
    editor.undo(); // back to [1]
    assert_eq!(editor.text, "a");
    editor.undo(); // back to [0]
    assert_eq!(editor.text, "");

    editor.redo(); // back to [1]
    assert_eq!(editor.text, "a");
    editor.redo(); // back to [2]
    assert_eq!(editor.text, "ab");
    editor.redo(); // back to [3]
    assert_eq!(editor.text, "abd");
    editor.redo(); // back to [4]
    assert_eq!(editor.text, "ab");
    editor.redo(); // back to [5]
    assert_eq!(editor.text, "abc");
    editor.redo(); // back to [9]: redo inner consecutive sequence of undos in batch
                   //              (undo are merged only when they are not the last action)
    assert_eq!(editor.text, "a");
    editor.redo(); // back to [10]
    assert_eq!(editor.text, "az");

    editor.add_char('1');
    editor.add_char('2');
    assert_eq!(editor.text, "az12");
    editor.undo();
    editor.undo();
    assert_eq!(editor.text, "az");
    editor.redo(); // undo is the last action, undo the undo only once
    assert_eq!(editor.text, "az1");
    editor.redo();
    assert_eq!(editor.text, "az12");
}
struct CommandString(Vec<(usize, char)>, Commands<(usize, char)>);
impl CommandString {
    fn new() -> Self {
        CommandString(vec![], Commands::new())
    }
    fn push(&mut self, c: char) {
        let l = self.0.len();
        self.0.push((l, c));
        self.1.push((l, c));
    }
    #[track_caller]
    fn undo(&mut self) {
        Self::apply(&mut self.0, self.1.undo());
    }
    #[track_caller]
    fn redo(&mut self) {
        Self::apply(&mut self.0, self.1.redo());
    }
    #[track_caller]
    fn apply<'a>(s: &mut Vec<(usize, char)>, it: impl Iterator<Item = Action<&'a (usize, char)>>) {
        for c in it {
            match c {
                Do(i) => {
                    assert_eq!(s.len(), i.0, "inconsitent push");
                    s.push(*i);
                }
                Undo(i) => {
                    assert_eq!(s.pop(), Some(*i), "inconsistent pop");
                }
            }
        }
    }
}
#[test]
fn command_sequence() {
    let mut c = CommandString::new();
    c.push('a');
    assert!(c.0 == [(0, 'a')]);
    assert!(c.1.len() == 1);
    c.undo();
    assert!(c.0.is_empty());
    c.redo();
    assert!(c.0 == [(0, 'a')]);
    c.push('b');
    c.push('c');
    assert!(c.0 == [(0, 'a'), (1, 'b'), (2, 'c')]);
    c.redo();
    assert!(c.0 == [(0, 'a'), (1, 'b'), (2, 'c')]);
    c.undo();
    assert!(c.0 == [(0, 'a'), (1, 'b')]);
    c.push('d');
    assert!(c.0 == [(0, 'a'), (1, 'b'), (2, 'd')]);
    c.push('e');
    assert!(c.0 == [(0, 'a'), (1, 'b'), (2, 'd'), (3, 'e')]);
    c.undo();
    assert!(c.0 == [(0, 'a'), (1, 'b'), (2, 'd')]);
    c.undo();
    assert!(c.0 == [(0, 'a'), (1, 'b')]);
    c.undo();
    assert!(c.0 == [(0, 'a'), (1, 'b'), (2, 'c')]);
    c.undo();
    assert!(c.0 == [(0, 'a'), (1, 'b')]);
    c.undo();
    assert!(c.0 == [(0, 'a')]);
    c.push('f');
    assert!(c.0 == [(0, 'a'), (1, 'f')]);
    c.undo();
    assert!(c.0 == [(0, 'a')]);
    c.undo();
    assert!(c.0 == [(0, 'a'), (1, 'b'), (2, 'd'), (3, 'e')]);
    c.redo();
    assert!(c.0 == [(0, 'a')]);
    c.undo();
    assert!(c.0 == [(0, 'a'), (1, 'b'), (2, 'd'), (3, 'e')]);
}
#[cfg(feature = "serde")]
#[test]
fn serde() {
    let mut commands = Commands::new();
    commands.push("a");
    let str = serde_json::to_string(&commands).unwrap();
    let commands: Commands<String> = serde_json::from_str(&str).unwrap();
    assert_eq!(*commands, ["a".to_owned().into()]);
}
