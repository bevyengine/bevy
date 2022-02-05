#[macro_export]
macro_rules! print_std140 {
    ($type:ty) => {
        println!(
            "{}",
            <$type as crevice::std140::AsStd140>::Output::debug_metrics()
        );
        println!();
        println!();
        println!(
            "{}",
            <$type as crevice::std140::AsStd140>::Output::debug_definitions()
        );
    };
}

#[macro_export]
macro_rules! print_std430 {
    ($type:ty) => {
        println!(
            "{}",
            <$type as crevice::std430::AsStd430>::Output::debug_metrics()
        );
        println!();
        println!();
        println!(
            "{}",
            <$type as crevice::std430::AsStd430>::Output::debug_definitions()
        );
    };
}

#[macro_export]
macro_rules! assert_std140 {
    ((size = $size:literal, align = $align:literal) $struct:ident {
        $( $field:ident: $offset:literal, )*
    }) => {{
        type Target = <$struct as crevice::std140::AsStd140>::Output;

        let mut fail = false;

        let actual_size = std::mem::size_of::<Target>();
        if actual_size != $size {
            fail = true;
            println!(
                "Invalid size for std140 struct {}\n\
                Expected: {}\n\
                Actual:   {}\n",
                stringify!($struct),
                $size,
                actual_size,
            );
        }

        let actual_alignment = <Target as crevice::std140::Std140>::ALIGNMENT;
        if actual_alignment != $align {
            fail = true;
            println!(
                "Invalid alignment for std140 struct {}\n\
                Expected: {}\n\
                Actual:   {}\n",
                stringify!($struct),
                $align,
                actual_alignment,
            );
        }

        $({
            let actual_offset = memoffset::offset_of!(Target, $field);
            if actual_offset != $offset {
                fail = true;
                println!(
                    "Invalid offset for field {}\n\
                    Expected: {}\n\
                    Actual:   {}\n",
                    stringify!($field),
                    $offset,
                    actual_offset,
                );
            }
        })*

        if fail {
            panic!("Invalid std140 result for {}", stringify!($struct));
        }
    }};
}

#[macro_export]
macro_rules! assert_std430 {
    ((size = $size:literal, align = $align:literal) $struct:ident {
        $( $field:ident: $offset:literal, )*
    }) => {{
        type Target = <$struct as crevice::std430::AsStd430>::Output;

        let mut fail = false;

        let actual_size = std::mem::size_of::<Target>();
        if actual_size != $size {
            fail = true;
            println!(
                "Invalid size for std430 struct {}\n\
                Expected: {}\n\
                Actual:   {}\n",
                stringify!($struct),
                $size,
                actual_size,
            );
        }

        let actual_alignment = <Target as crevice::std430::Std430>::ALIGNMENT;
        if actual_alignment != $align {
            fail = true;
            println!(
                "Invalid alignment for std430 struct {}\n\
                Expected: {}\n\
                Actual:   {}\n",
                stringify!($struct),
                $align,
                actual_alignment,
            );
        }

        $({
            let actual_offset = memoffset::offset_of!(Target, $field);
            if actual_offset != $offset {
                fail = true;
                println!(
                    "Invalid offset for std430 field {}\n\
                    Expected: {}\n\
                    Actual:   {}\n",
                    stringify!($field),
                    $offset,
                    actual_offset,
                );
            }
        })*

        if fail {
            panic!("Invalid std430 result for {}", stringify!($struct));
        }
    }};
}
