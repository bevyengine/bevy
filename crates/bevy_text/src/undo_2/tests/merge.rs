use undo_2::*;

#[test]
fn merge() {
    use std::ops::ControlFlow;
    use undo_2::CommandItem;
    use undo_2::IterRealized;
    use undo_2::Merge;
    #[derive(Eq, PartialEq, Debug)]
    enum Command {
        A,
        B,
        C,
        AB,
    }
    use Command::*;
    fn is_ab(mut it: IterRealized<'_, Command>) -> (bool, IterRealized<'_, Command>) {
        let cond = it.next() == Some(&Command::B) && it.next() == Some(&Command::A);
        (cond, it)
    }
    fn parse(
        start: IterRealized<'_, Command>,
    ) -> ControlFlow<Option<Merge<'_, Command>>, Option<Merge<'_, Command>>> {
        if let (true, end) = is_ab(start.clone()) {
            ControlFlow::Continue(Some(Merge {
                start,
                end,
                command: Some(Command::AB),
            }))
        } else {
            ControlFlow::Continue(None)
        }
    }
    {
        let mut commands = Commands::new();
        commands.push(Command::A);
        commands.push(Command::B);

        commands.merge(parse);
        assert_eq!(*commands, [CommandItem::Command(Command::AB)]);
    }
    {
        let mut commands = Commands::new();
        commands.push(Command::A);
        commands.push(Command::C);
        commands.push(Command::B);
        commands.push(Command::A);
        commands.push(Command::B);

        commands.merge(parse);
        assert_eq!(
            *commands,
            [
                CommandItem::Command(A),
                CommandItem::Command(C),
                CommandItem::Command(B),
                CommandItem::Command(AB)
            ]
        );
    }
    {
        let mut commands = Commands::new();
        commands.push(Command::A);
        commands.push(Command::C);
        commands.push(Command::A);
        commands.push(Command::B);
        commands.push(Command::B);
        commands.push(Command::A);
        commands.push(Command::B);

        commands.merge(parse);
        assert_eq!(
            *commands,
            [
                CommandItem::Command(A),
                CommandItem::Command(C),
                CommandItem::Command(AB),
                CommandItem::Command(B),
                CommandItem::Command(AB)
            ]
        );
    }
    {
        let mut commands = Commands::new();
        commands.push(Command::A);
        commands.push(Command::B);
        commands.push(Command::A);
        commands.push(Command::B);

        commands.merge(parse);
        assert_eq!(
            &*commands,
            &[CommandItem::Command(AB), CommandItem::Command(AB)]
        );
    }
}
