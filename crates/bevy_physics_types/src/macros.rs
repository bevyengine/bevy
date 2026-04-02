macro_rules! make_global {
    (
        $(#[$meta:meta])*
        $name:ident($ty:ty) = $default:expr;
        $($rest:tt)*
    ) => {
        #[derive(bevy_ecs::resource::Resource)]
        $(#[$meta])*
        pub struct $name(pub $ty);

        impl Default for $name {
            fn default() -> Self {
                $name($default)
            }
        }
    };

    (
        $(#[$meta:meta])*
        $name:ident($ty:ty);
        $($rest:tt)*
    ) => {
        #[derive(bevy_ecs::resource::Resource)]
        $(#[$meta])*
        pub struct $name(pub $ty);
    };
}

macro_rules! make_attribute {
    (
        $(#[$meta:meta])*
        $name:ident($ty:ty) = $default:expr;
        $($rest:tt)*
    ) => {
        #[derive(bevy_ecs::component::Component)]
        $(#[$meta])*
        pub struct $name(pub $ty);

        impl Default for $name {
            fn default() -> Self {
                $name($default)
            }
        }
    };

    (
        $(#[$meta:meta])*
        $name:ident($ty:ty);
        $($rest:tt)*
    ) => {
        #[derive(bevy_ecs::component::Component)]
        $(#[$meta])*
        pub struct $name(pub $ty);
    };
}

macro_rules! make_marker {
    (
        $(#[$meta:meta])*
        $name:ident;
        $($rest:tt)*
    ) => {
        #[derive(bevy_ecs::component::Component)]
        $(#[$meta])*
        pub struct $name;

        impl Default for $name {
            fn default() -> Self {
                $name
            }
        }
    };
}

macro_rules! make_asset {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        // $(#[$meta])*
        // pub struct $name;
    };
}

macro_rules! make_enum {
    (
        $(#[$meta:meta])*
        $trait_name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident = $value:literal
            ),* $(,)?
        }
        $($attr_name:ident = $attr_value:expr)*
    ) => {
        $(#[$meta])*
        pub trait $trait_name {}

        $(
            $(#[$variant_meta])*
            #[derive(bevy_ecs::component::Component)]
            pub struct $variant;

            impl $trait_name for $variant {}

            impl Default for $variant {
                fn default() -> Self {
                    $variant
                }
            }
        )*
    };
}

macro_rules! make_collection {
    (
        $(#[$meta:meta])*
        $member:ident -> $collection:ident($ty:ty);
        $($attr_name:ident = $attr_value:expr)*
    ) => {
        $(#[$meta])*
        #[derive(bevy_ecs::component::Component)]
        #[component(immutable)]
        pub struct $member;

        $(#[$meta])*
        #[derive(bevy_ecs::component::Component, Clone)]
        pub struct $collection(pub $ty);

        impl Default for $collection {
            fn default() -> Self {
                $collection(Default::default())
            }
        }
    };
}
