#[doc(hidden)]
#[macro_export]
macro_rules! bind_group_descriptor_acc {
    (@d texture($e:expr)) => ($crate::render_resource::BindingResource::TextureView($e));
    (@d sampler($e:expr)) => ($crate::render_resource::BindingResource::Sampler($e));
    (@d buffer($e:expr)) => ($e);
    (@d $_:ident($e:expr)) => (compile_error!("only 'texture', 'sampler', and 'buffer' bindings are supported"));

    (@acc $index:ident, [$($acc:tt)*] $fn:ident ($e:expr) $(, $($rest:tt)* )?) => {
        $crate::bind_group_descriptor_acc!(@acc $index,
            [
                $($acc)*
                $crate::render_resource::BindGroupEntry {
                    binding: {$index += 1; $index-1},
                    resource: $crate::bind_group_descriptor_acc!(@d $fn($e)),
                },
            ] $($($rest)*)?
        )
    };

    // Nothing left
    (@acc $index:ident,
        [$($output:tt)*] $(,)?
    ) => (
        [ $($output)* ]
    );
}

/// TODO: Docs
#[macro_export]
macro_rules! bind_group_descriptor {
    ($label:expr, $layout:expr, $($fun:tt)*) => {{
        let mut index = 0;
        &$crate::render_resource::BindGroupDescriptor {
            label: Some($label),
            layout: $layout,
            entries: &$crate::bind_group_descriptor_acc!(@acc index, [] $($fun)*)
        }
    }};
}
