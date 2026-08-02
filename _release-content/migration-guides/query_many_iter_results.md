---
title: `QueryManyIter` and all other `*iter_many*` iterators now iterate over `Result` instead of `QueryData::Item`.
pull_requests: [25200]
---

The following iterators now return `Result<QueryData::Item<'w, 's>, QueryEntityError>`
instead of `QueryData::Item<'w, 's>` as the `Iterator::Item`.

- `QueryManyIter`
- `QueryManyUniqueIter`
- `QuerySortedManyIter`

These iterators are created by the following methods.

- `Query::iter_many`
- `Query::iter_many_mut`
- `Query::iter_many_unique`
- `Query::iter_many_unique_mut`
- `QueryManyIter::sort*`

Likewise, the following parallel iterator methods now have `Result<QueryData::Item<'w, 's>, QueryEntityError>`
instead of `QueryData::Item<'w, 's>` as the closure argument.

- `QueryParManyIter::for_each`
- `QueryParManyIter::for_each_init`, 
- `QueryParManyUniqueIter::for_each`
- `QueryParManyUniqueIter::for_each_init`, 

These parallel iterators are created by the following methods.

- `Query::par_iter_many`
- `Query::par_iter_many_unique`
- `Query::par_iter_many_unique_mut`

These changes were made to allow for full flexibility in handling not matched or not spawned entities.
This allows easier debugging by making these errors visible through a panic or logging
instead of only giving the option to silently ignore these errors.

For `QueryManyIter` and `QueryManyUniqueIter` users can migrate using

- `QueryManyIter::matched`
- `QueryManyUniqueIter::matched`

which provide the same behavior as before.

```rust
// 0.19
fn my_system(entity_list: Res<MyEntityList>, my_component_query: Query<&MyComponent>) {
    for my_component in my_component_query.iter_many(entity_list.iter()) {
        // ...
    }
}

// 0.20
fn my_system(entity_list: Res<MyEntityList>, my_component_query: Query<&MyComponent>) {
    for my_component in my_component_query.iter_many(entity_list.iter()).matched() {
        // ...
    }
}

// 0.19
fn my_mutation_system(entity_list: Res<MyEntityList>, mut my_component_query: Query<&mut MyComponent>) {
    let mut iter = my_component_query.iter_many_mut(entity_list.iter());
    while let Some(my_component) in iter.fetch_next() {
        // ...
    }
}

// 0.20
fn my_mutation_system(entity_list: Res<MyEntityList>, mut my_component_query: Query<&mut MyComponent>) {
    let mut iter = my_component_query.iter_many_mut(entity_list.iter()).matched();
    while let Some(my_component) in iter.fetch_next() {
        // ...
    }
}
```

For `QuerySortedManyIter` you can only use `.flat_map(Result::ok)`.

```rust
// 0.19
fn my_system(entity_list: Res<MyEntityList>, my_component_query: Query<&MyComponent>) {
    for my_component in my_component_query.iter_many(entity_list.iter())
        .sort::<&MyComponent>()
    {
        // ...
    }
}

// 0.20
fn my_system(entity_list: Res<MyEntityList>, my_component_query: Query<&MyComponent>) {
    for my_component in my_component_query.iter_many(entity_list.iter())
        .sort::<&MyComponent>()
        .flat_map(Result::ok)
    {
        // ...
    }
}
```

For `QueryParManyIter` and `QueryParManyUniqueIter` you need to return early when there is an error
to get the same `matched` behavior as before.

```rust
// 0.19
fn my_system(entity_list: Res<MyEntityList>, my_component_query: Query<&MyComponent>) {
    my_component_query.par_iter_many(entity_list.iter()).for_each(|my_component| {
        // ...
    });
}

// 0.20
fn my_system(entity_list: Res<MyEntityList>, my_component_query: Query<&MyComponent>) {
    my_component_query.par_iter_many(entity_list.iter()).for_each(|my_component| {
        let Ok(my_component) = my_component else {
            return;
        };
        // ...
    });
}
```
