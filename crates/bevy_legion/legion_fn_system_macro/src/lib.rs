extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::Ident;


#[proc_macro]
pub fn impl_fn_systems(_input: TokenStream) -> TokenStream {
    let max_resources = 8;
    let max_views = 8;
    let resources = (0..max_resources)
        .map(|i| Ident::new(&format!("R{}", i), Span::call_site()))
        .collect::<Vec<Ident>>();
    let resource_vars = (0..max_resources)
        .map(|i| Ident::new(&format!("r{}", i), Span::call_site()))
        .collect::<Vec<Ident>>();
    let views = (0..max_views)
        .map(|i| Ident::new(&format!("V{}", i), Span::call_site()))
        .collect::<Vec<Ident>>();
    let view_vars = (0..max_views)
        .map(|i| Ident::new(&format!("v{}", i), Span::call_site()))
        .collect::<Vec<Ident>>();
    let filter_idents = (0..max_views)
        .map(|i| Ident::new(&format!("VF{}", i), Span::call_site()))
        .collect::<Vec<Ident>>();

    let mut tokens = TokenStream::new();

    let command_buffer = vec![Ident::new("CommandBuffer", Span::call_site())];
    let command_buffer_var = vec![Ident::new("_command_buffer", Span::call_site())];
    for resource_count in 0..max_resources {
        let resource = &resources[0..resource_count];
        let resource_var = &resource_vars[0..resource_count];

        let resource_tuple = if resource_count == 0 {
            quote!{ ()} 
        } else if resource_count == 1 {
            quote!{ #(#resource),* } 
        } else {
            quote!{ (#(#resource),*) } 
        };

        let resource_var_tuple = if resource_count == 0 {
            quote!{ ()} 
        } else if resource_count == 1 {
            quote!{ #(#resource_var),* } 
        } else {
            quote!{ (#(#resource_var),*) } 
        };

        let resource_access = if resource_count == 0 {
            quote! { Access::default() }
        }else {
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

        for view_count in 0..max_views {
            let view = &views[0..view_count];
            let view_var = &view_vars[0..view_count];
            let filter = &filter_idents[0..view_count];

            let view_tuple = if view_count == 0 {
                quote!{ ()} 
            } else if view_count == 1 {
                quote!{ #(#view),* } 
            } else {
                quote!{ (#(#view),*) } 
            };

            let component_access = if view_count == 0 {
                quote! { Access::default() }
            }else {
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
                    #(#filter),*,
                > }
            } else {
                quote! { SystemQuery<
                    (#(#view),*),
                    EntityFilterTuple<
                        And<(
                            #(<#filter as EntityFilter>::ArchetypeFilter),*
                        )>,
                        And<(
                            #(<#filter as EntityFilter>::ChunksetFilter),*
                        )>,
                        And<(
                            #(<#filter as EntityFilter>::ChunkFilter),*
                        )>,
                    >
                >   }
            };

            let query = if view_count == 0 {
                quote!{()}
            } else {
                quote!{<#view_tuple>::query()}
            };

            for command_buffer_index in 0..2 {
                let command_buffer = &command_buffer[0..command_buffer_index];
                let command_buffer_var = &command_buffer_var[0..command_buffer_index];

                let run_fn = if view_count == 0 {
                    quote! { self(#(#command_buffer_var,)*#(#resource_var),*) }
                } else {
                    quote! {
                        for (#(#view_var),*) in _query.iter_mut(_world) {
                            self(#(#command_buffer_var,)*#(#resource_var.clone(),)* #(#view_var),*);
                        }
                    }
                };

                tokens.extend(TokenStream::from(quote! {
                    impl<'a,
                    Func,
                    #(#resource: ResourceSet<PreparedResources = #resource> + 'static + Clone,)*
                    #(#view: for<'b> View<'b> + DefaultFilter<Filter = #filter> + ViewElement,
                    #filter: EntityFilter + Sync + 'static),*
                > IntoSystem<'a, (#(#command_buffer)*), (#(#resource,)*), (#(#view,)*)> for Func
                    where
                        Func: FnMut(#(&mut #command_buffer,)* #(#resource,)* #(#view),*) + Send + Sync + 'static,
                        #(<#view as View<'a>>::Iter: Iterator<Item = #view>),*
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
