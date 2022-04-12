use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;

#[derive(Component, Eq, PartialEq, Debug)]
struct Foo(u32);

fn main() {
    let mut world = World::default();
    let e = world.spawn().insert(Foo(10_u32)).id();

    let mut system_state = SystemState::<Query<&mut Foo>>::new(&mut world);
    {
        let mut query = system_state.get_mut(&mut world);
        dbg!("hi");
        {
            let data: &Foo = query.get(e).unwrap();
            let mut data2: Mut<Foo> = query.get_mut(e).unwrap();
            assert_eq!(data, &mut *data2); // oops UB
        }

        {
            let mut data2: Mut<Foo> = query.get_mut(e).unwrap();
            let data: &Foo = query.get(e).unwrap();
            assert_eq!(data, &mut *data2); // oops UB
        }

        {
            let data: &Foo = query.get_component::<Foo>(e).unwrap();
            let mut data2: Mut<Foo> = query.get_component_mut(e).unwrap();
            assert_eq!(data, &mut *data2); // oops UB
        }

        {
            let mut data2: Mut<Foo> = query.get_component_mut(e).unwrap();
            let data: &Foo = query.get_component::<Foo>(e).unwrap();
            assert_eq!(data, &mut *data2); // oops UB
        }

        {
            let data: &Foo = query.single();
            let mut data2: Mut<Foo> = query.single_mut();
            assert_eq!(data, &mut *data2); // oops UB
        }

        {
            let mut data2: Mut<Foo> = query.single_mut();
            let data: &Foo = query.single();
            assert_eq!(data, &mut *data2); // oops UB
        }

        {
            let data: &Foo = query.get_single().unwrap();
            let mut data2: Mut<Foo> = query.get_single_mut().unwrap();
            assert_eq!(data, &mut *data2); // oops UB
        }

        {
            let mut data2: Mut<Foo> = query.get_single_mut().unwrap();
            let data: &Foo = query.get_single().unwrap();
            assert_eq!(data, &mut *data2); // oops UB
        }

        {
            let data: &Foo = query.iter().next().unwrap();
            let mut data2: Mut<Foo> = query.iter_mut().next().unwrap();
            assert_eq!(data, &mut *data2); // oops UB
        }

        {
            let mut data2: Mut<Foo> = query.iter_mut().next().unwrap();
            let data: &Foo = query.iter().next().unwrap();
            assert_eq!(data, &mut *data2); // oops UB
        }

        {
            let mut opt_data: Option<&Foo> = None;
            let mut opt_data_2: Option<Mut<Foo>> = None;
            query.for_each(|data| opt_data = Some(data));
            query.for_each_mut(|data| opt_data_2 = Some(data));
            assert_eq!(opt_data.unwrap(), &mut *opt_data_2.unwrap()); // oops UB
        }

        {
            let mut opt_data_2: Option<Mut<Foo>> = None;
            let mut opt_data: Option<&Foo> = None;
            query.for_each_mut(|data| opt_data_2 = Some(data));
            query.for_each(|data| opt_data = Some(data));
            assert_eq!(opt_data.unwrap(), &mut *opt_data_2.unwrap()); // oops UB
        }
        dbg!("bye");
    }
}
