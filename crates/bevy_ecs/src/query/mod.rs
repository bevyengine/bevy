mod access;
mod fetch;
mod filter;
mod iter;
mod state;

pub use access::*;
pub use fetch::*;
pub use filter::*;
pub use iter::*;
pub use state::*;

#[cfg(test)]
mod tests {
    use crate::{
        component::{ComponentDescriptor, StorageType},
        world::World,
    };

    #[derive(Debug, Eq, PartialEq)]
    struct A(usize);
    #[derive(Debug, Eq, PartialEq)]
    struct B(usize);

    #[test]
    fn query() {
        let mut world = World::new();
        world.spawn().insert_bundle((A(1), B(1)));
        world.spawn().insert_bundle((A(2),));
        let values = world.query::<&A>().iter(&world).collect::<Vec<&A>>();
        assert_eq!(values, vec![&A(1), &A(2)]);

        for (_a, mut b) in world.query::<(&A, &mut B)>().iter_mut(&mut world) {
            b.0 = 3;
        }
        let values = world.query::<&B>().iter(&world).collect::<Vec<&B>>();
        assert_eq!(values, vec![&B(3)]);
    }

    #[test]
    fn multi_storage_query() {
        let mut world = World::new();
        world
            .register_component(ComponentDescriptor::new::<A>(StorageType::SparseSet))
            .unwrap();

        world.spawn().insert_bundle((A(1), B(2)));
        world.spawn().insert_bundle((A(2),));

        let values = world.query::<&A>().iter(&world).collect::<Vec<&A>>();
        assert_eq!(values, vec![&A(1), &A(2)]);

        for (_a, mut b) in world.query::<(&A, &mut B)>().iter_mut(&mut world) {
            b.0 = 3;
        }

        let values = world.query::<&B>().iter(&world).collect::<Vec<&B>>();
        assert_eq!(values, vec![&B(3)]);
    }
}
