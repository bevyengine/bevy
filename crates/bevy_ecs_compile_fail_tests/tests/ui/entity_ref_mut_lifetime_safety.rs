use bevy_ecs::prelude::*;

#[derive(Component, Eq, PartialEq, Debug)]
struct A(Box<usize>);

#[derive(Component)]
struct B;

fn main() {
    let mut world = World::default();
    let e = world.spawn(A(Box::new(10_usize))).id();

    let mut e_mut = world.entity_mut(e);

    {
        let gotten: &A = e_mut.get::<A>().unwrap();
        let gotten2: A = e_mut.take::<A>().unwrap();
        assert_eq!(gotten, &gotten2); // oops UB
    }

    e_mut.insert(A(Box::new(12_usize)));

    {
        let mut gotten: Mut<A> = e_mut.get_mut::<A>().unwrap();
        let mut gotten2: A = e_mut.take::<A>().unwrap();
        assert_eq!(&mut *gotten, &mut gotten2); // oops UB
    }

    e_mut.insert(A(Box::new(14_usize)));

    {
        let gotten: &A = e_mut.get::<A>().unwrap();
        e_mut.despawn();
        assert_eq!(gotten, &A(Box::new(14_usize))); // oops UB
    }

    let e = world.spawn(A(Box::new(16_usize))).id();
    let mut e_mut = world.entity_mut(e);

    {
        let gotten: &A = e_mut.get::<A>().unwrap();
        let gotten_mut: Mut<A> = e_mut.get_mut::<A>().unwrap();
        assert_eq!(gotten, &*gotten_mut); // oops UB
    }

    {
        let gotten_mut: Mut<A> = e_mut.get_mut::<A>().unwrap();
        let gotten: &A = e_mut.get::<A>().unwrap();
        assert_eq!(gotten, &*gotten_mut); // oops UB
    }

    {
        let gotten: &A = e_mut.get::<A>().unwrap();
        e_mut.insert::<B>(B);
        assert_eq!(gotten, &A(Box::new(16_usize))); // oops UB
        e_mut.remove::<B>();
    }

    {
        let mut gotten_mut: Mut<A> = e_mut.get_mut::<A>().unwrap();
        e_mut.insert::<B>(B);
        assert_eq!(&mut *gotten_mut, &mut A(Box::new(16_usize))); // oops UB
    }
}
