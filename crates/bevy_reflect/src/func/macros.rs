macro_rules! impl_function_traits {
    (
        $name: ty
        $(;
            <
                $($T: ident $(: $T1: tt $(+ $T2: tt)*)?),*
            >
        )?
        $(
            [
                $(const $N: ident : $size: ident),*
            ]
        )?
        $(
            where
                $($U: ty $(: $U1: tt $(+ $U2: tt)*)?),*
        )?
    ) => {
        $crate::func::args::impl_get_ownership!(
            $name
            $(;
                <
                    $($T $(: $T1 $(+ $T2)*)?),*
                >
            )?
            $(
                [
                    $(const $N : $size),*
                ]
            )?
            $(
                where
                    $($U $(: $U1 $(+ $U2)*)?),*
            )?
        );
        $crate::func::args::impl_from_arg!(
            $name
            $(;
                <
                    $($T $(: $T1 $(+ $T2)*)?),*
                >
            )?
            $(
                [
                    $(const $N : $size),*
                ]
            )?
            $(
                where
                    $($U $(: $U1 $(+ $U2)*)?),*
            )?
        );
        $crate::func::impl_into_return!(
            $name
            $(;
                <
                    $($T $(: $T1 $(+ $T2)*)?),*
                >
            )?
            $(
                [
                    $(const $N : $size),*
                ]
            )?
            $(
                where
                    $($U $(: $U1 $(+ $U2)*)?),*
            )?
        );
    };
}

pub(crate) use impl_function_traits;
