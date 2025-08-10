#[test]
fn merge() {
    use crate::undo_2::CommandItem;
    use crate::undo_2::Commands;
    use crate::undo_2::IterRealized;
    use crate::undo_2::Merge;
    use core::ops::ControlFlow;
    #[derive(Eq, PartialEq, Debug)]
    enum Command {
        A,
        B,
        C,
        AB,
    }
    use Command::*;
    fn is_ab(mut it: IterRealized<'_, Command>) -> (bool, IterRealized<'_, Command>) {
        let cond = it.next() == Some(&B) && it.next() == Some(&A);
        (cond, it)
    }
    fn parse(
        start: IterRealized<'_, Command>,
    ) -> ControlFlow<Option<Merge<'_, Command>>, Option<Merge<'_, Command>>> {
        if let (true, end) = is_ab(start.clone()) {
            ControlFlow::Continue(Some(Merge {
                start,
                end,
                command: Some(AB),
            }))
        } else {
            ControlFlow::Continue(None)
        }
    }
    {
        let mut commands = Commands::new();
        commands.push(A);
        commands.push(B);

        commands.merge(parse);
        assert_eq!(*commands, [CommandItem::Command(AB)]);
    }
    {
        let mut commands = Commands::new();
        commands.push(A);
        commands.push(C);
        commands.push(B);
        commands.push(A);
        commands.push(B);

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
        commands.push(A);
        commands.push(C);
        commands.push(A);
        commands.push(B);
        commands.push(B);
        commands.push(A);
        commands.push(B);

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
        commands.push(A);
        commands.push(B);
        commands.push(A);
        commands.push(B);

        commands.merge(parse);
        assert_eq!(
            &*commands,
            &[CommandItem::Command(AB), CommandItem::Command(AB)]
        );
    }
}
