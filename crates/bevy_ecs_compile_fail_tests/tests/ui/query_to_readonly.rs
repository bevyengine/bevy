use bevy_ecs::prelude::*;

#[derive(Component, Debug)]
struct Foo;

fn for_loops(mut query: Query<&mut Foo>) {
    // this should fail to compile
    for _ in query.iter_mut() {
        for _ in query.to_readonly().iter() {}
        //~^ E0502
    }

    // this should fail to compile
    for _ in query.to_readonly().iter() {
        for _ in query.iter_mut() {}
        //~^ E0502
    }

    // this should *not* fail to compile
    for _ in query.to_readonly().iter() {
        for _ in query.to_readonly().iter() {}
    }

    // this should *not* fail to compile
    for _ in query.to_readonly().iter() {
        for _ in query.iter() {}
    }

    // this should *not* fail to compile
    for _ in query.iter() {
        for _ in query.to_readonly().iter() {}
    }
}

fn single_mut_query(mut query: Query<&mut Foo>) {
    // this should fail to compile
    {
        let mut mut_foo = query.single_mut();

        // This solves "temporary value dropped while borrowed"
        let readonly_query = query.to_readonly();
        //~^ E0502

        let ref_foo = readonly_query.single();
    
        *mut_foo = Foo;

        println!("{ref_foo:?}");
    }

    // this should fail to compile
    {
        // This solves "temporary value dropped while borrowed"
        let readonly_query = query.to_readonly();

        let ref_foo = readonly_query.single();

        let mut mut_foo = query.single_mut();
        //~^ E0502

        println!("{ref_foo:?}");

        *mut_foo = Foo;
    }

    // this should *not* fail to compile
    {
        // This solves "temporary value dropped while borrowed"
        let readonly_query = query.to_readonly();

        let readonly_foo = readonly_query.single();

        let query_foo = query.single();

        println!("{readonly_foo:?}, {query_foo:?}");
    }
}
