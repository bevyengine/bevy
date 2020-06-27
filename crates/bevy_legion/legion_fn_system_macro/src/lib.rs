extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::Ident;

fn tuple(idents: &[Ident]) -> proc_macro2::TokenStream {
    if idents.len() == 0 {
        quote! { ()}
    } else if idents.len() == 1 {
        quote! { #(#idents),* }
    } else {
        quote! { (#(#idents),*) }
    }
}

fn get_idents(fmt_string: fn(usize) -> String, count: usize) -> Vec<Ident> {
    (0..=count)
        .map(|i| Ident::new(&fmt_string(i), Span::call_site()))
        .collect::<Vec<Ident>>()
}

#[proc_macro]
pub fn impl_fn_systems(_input: TokenStream) -> TokenStream {
    let max_resources = 8;
    let max_views = 8;
    let resources = get_idents(|i| format!("R{}", i), max_resources);
    let resource_vars = get_idents(|i| format!("r{}", i), max_resources);
    let views = get_idents(|i| format!("V{}", i), max_views);
    let view_vars = get_idents(|i| format!("v{}", i), max_views);

    let mut tokens = TokenStream::new();

    let command_buffer = vec![Ident::new("CommandBuffer", Span::call_site())];
    let command_buffer_var = vec![Ident::new("_command_buffer", Span::call_site())];
    for resource_count in 0..=max_resources {
        let resource = &resources[0..resource_count];
        let resource_var = &resource_vars[0..resource_count];

        let resource_tuple = tuple(resource);
        let resource_var_tuple = tuple(resource_var);

        let resource_access = if resource_count == 0 {
            quote! { Access::default() }
        } else {
            quote! {{
                let mut resource_access: Access<ResourceTypeId> = Access::default();
                resource_access
                    .reads
                    .extend(<#resource_tuple as ResourceSet>::read_types().iter());
                resource_access
                    .writes
                    .extend(<#resource_tuple as ResourceSet>::write_types().iter());
                resource_access
            }}
        };

        for view_count in 0..=max_views {
            let view = &views[0..view_count];
            let view_var = &view_vars[0..view_count];

            let view_tuple = tuple(view);

            let component_access = if view_count == 0 {
                quote! { Access::default() }
            } else {
                quote! {{
                    let mut component_access: Access<ComponentTypeId> = Access::default();
                    component_access
                        .reads
                        .extend(<#view_tuple as View>::read_types().iter());
                    component_access
                        .writes
                        .extend(<#view_tuple as View>::write_types().iter());
                    component_access
                }}
            };

            let system_query = if view_count == 0 {
                quote! { () }
            } else if view_count == 1 {
                quote! { SystemQuery<
                    #(#view),*,
                    #(<#view as DefaultFilter>::Filter),*,
                > }
            } else {
                quote! { SystemQuery<
                    (#(#view),*),
                    EntityFilterTuple<
                        And<(
                            #(<<#view as DefaultFilter>::Filter as EntityFilter>::ArchetypeFilter),*
                        )>,
                        And<(
                            #(<<#view as DefaultFilter>::Filter as EntityFilter>::ChunksetFilter),*
                        )>,
                        And<(
                            #(<<#view as DefaultFilter>::Filter as EntityFilter>::ChunkFilter),*
                        )>,
                    >
                >   }
            };

            let query = if view_count == 0 {
                quote! {()}
            } else {
                quote! {<#view_tuple>::query()}
            };

            for command_buffer_index in 0..2 {
                let command_buffer = &command_buffer[0..command_buffer_index];
                let command_buffer_var = &command_buffer_var[0..command_buffer_index];

                let run_fn = if view_count == 0 {
                    quote! { self(#(#resource_var,)* #(#command_buffer_var,)*) }
                } else {
                    quote! {
                        for (#(#view_var),*) in _query.iter_mut(_world) {
                            self(#(#resource_var.clone(),)* #(#command_buffer_var,)* #(#view_var),*);
                        }
                    }
                };

                tokens.extend(TokenStream::from(quote! {
                    impl<'a,
                        Func,
                        #(#resource: ResourceSet<PreparedResources = #resource> + 'static + Clone,)*
                        #(#view: for<'b> View<'b> + DefaultFilter + ViewElement,)*
                    > IntoSystem<(#(#command_buffer)*), (#(#resource,)*), (#(#view,)*), ()> for Func
                    where
                        Func: FnMut(#(#resource,)* #(&mut #command_buffer,)* #(#view),*) + Send + Sync + 'static,
                        #(<#view as View<'a>>::Iter: Iterator<Item = #view>,
                        <#view as DefaultFilter>::Filter: Sync),*
                    {
                        fn system_id(mut self, id: SystemId) -> Box<dyn Schedulable> {
                            let resource_access: Access<ResourceTypeId> = #resource_access;
                            let component_access: Access<ComponentTypeId> = #component_access;

                            let run_fn = FuncSystemFnWrapper(
                                move |_command_buffer,
                                    _world,
                                    _resources: #resource_tuple,
                                    _query: &mut #system_query
                                | {
                                    let #resource_var_tuple = _resources;
                                    #run_fn
                                },
                                PhantomData,
                            );

                            Box::new(FuncSystem {
                                name: id,
                                queries: AtomicRefCell::new(#query),
                                access: SystemAccess {
                                    resources: resource_access,
                                    components: component_access,
                                    tags: Access::default(),
                                },
                                archetypes: ArchetypeAccess::Some(BitSet::default()),
                                _resources: PhantomData::<#resource_tuple>,
                                command_buffer: FxHashMap::default(),
                                run_fn: AtomicRefCell::new(run_fn),
                            })
                        }

                        fn system_named(self, name: &'static str) -> Box<dyn Schedulable> {
                            self.system_id(name.into())
                        }

                        fn system(self) -> Box<dyn Schedulable> {
                            self.system_id(std::any::type_name::<Self>().to_string().into())
                        }
                    }
                }));
            }
        }
    }

    tokens
}

#[proc_macro]
pub fn impl_fn_query_systems(_input: TokenStream) -> TokenStream {
    let max_resources = 8;
    let max_queries = 4;

    let resources = get_idents(|i| format!("R{}", i), max_resources);
    let resource_vars = get_idents(|i| format!("r{}", i), max_resources);
    let views = get_idents(|i| format!("V{}", i), max_queries);
    let query_vars = get_idents(|i| format!("q{}", i), max_queries);

    let mut tokens = TokenStream::new();

    let command_buffer = vec![Ident::new("CommandBuffer", Span::call_site())];
    let command_buffer_var = vec![Ident::new("_command_buffer", Span::call_site())];
    for resource_count in 0..=max_resources {
        let resource = &resources[0..resource_count];
        let resource_var = &resource_vars[0..resource_count];

        let resource_tuple = tuple(resource);
        let resource_var_tuple = tuple(resource_var);

        let resource_access = if resource_count == 0 {
            quote! { Access::default() }
        } else {
            quote! {{
                let mut resource_access: Access<ResourceTypeId> = Access::default();
                resource_access
                    .reads
                    .extend(<#resource_tuple as ResourceSet>::read_types().iter());
                resource_access
                    .writes
                    .extend(<#resource_tuple as ResourceSet>::write_types().iter());
                resource_access
            }}
        };

        for query_count in 1..=max_queries {
            let view = &views[0..query_count];
            let query_var = &query_vars[0..query_count];

            let view_tuple = tuple(view);
            let query_var_tuple = tuple(query_var);

            let component_access = if query_count == 0 {
                quote! { Access::default() }
            } else {
                quote! {{
                    let mut component_access: Access<ComponentTypeId> = Access::default();
                    component_access
                        .reads
                        .extend(<#view_tuple as View>::read_types().iter());
                    component_access
                        .writes
                        .extend(<#view_tuple as View>::write_types().iter());
                    component_access
                }}
            };

            for command_buffer_index in 0..2 {
                let command_buffer = &command_buffer[0..command_buffer_index];
                let command_buffer_var = &command_buffer_var[0..command_buffer_index];

                let view_tuple_avoid_type_collision = if query_count == 1 {
                    quote! {(#(#view)*,)}
                } else {
                    quote! {(#(#view,)*)}
                };

                tokens.extend(TokenStream::from(quote! {
                    impl<Func,
                        #(#resource: ResourceSet<PreparedResources = #resource> + 'static + Clone,)*
                        #(#view: for<'b> View<'b> + DefaultFilter + ViewElement),*
                    > IntoSystem<(#(#command_buffer)*), (#(#resource,)*), (), #view_tuple_avoid_type_collision> for Func
                    where
                        Func: FnMut(#(#resource,)*#(&mut #command_buffer,)* &mut SubWorld, #(&mut SystemQuery<#view, <#view as DefaultFilter>::Filter>),*) + Send + Sync + 'static,
                        #(<#view as DefaultFilter>::Filter: Sync),*
                    {
                        fn system_id(mut self, id: SystemId) -> Box<dyn Schedulable> {
                            let resource_access: Access<ResourceTypeId> = #resource_access;
                            let component_access: Access<ComponentTypeId> = #component_access;

                            let run_fn = FuncSystemFnWrapper(
                                move |_command_buffer,
                                    _world,
                                    _resources: #resource_tuple,
                                    _queries: &mut (#(SystemQuery<#view, <#view as DefaultFilter>::Filter>),*)
                                | {
                                    let #resource_var_tuple = _resources;
                                    let #query_var_tuple = _queries;
                                    self(#(#resource_var,)*#(#command_buffer_var,)*_world,#(#query_var),*)
                                },
                                PhantomData,
                            );

                            Box::new(FuncSystem {
                                name: id,
                                queries: AtomicRefCell::new((#(<#view>::query()),*)),
                                access: SystemAccess {
                                    resources: resource_access,
                                    components: component_access,
                                    tags: Access::default(),
                                },
                                archetypes: ArchetypeAccess::Some(BitSet::default()),
                                _resources: PhantomData::<#resource_tuple>,
                                command_buffer: FxHashMap::default(),
                                run_fn: AtomicRefCell::new(run_fn),
                            })
                        }

                        fn system_named(self, name: &'static str) -> Box<dyn Schedulable> {
                            self.system_id(name.into())
                        }

                        fn system(self) -> Box<dyn Schedulable> {
                            self.system_id(std::any::type_name::<Self>().to_string().into())
                        }
                    }
                }));
            }
        }
    }
    tokens
}
