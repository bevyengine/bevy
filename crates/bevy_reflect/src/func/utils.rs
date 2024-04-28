use std::any::TypeId;

use crate::func::function::FunctionResult;
use crate::Reflect;

pub(super) fn to_function_result<R: Reflect>(value: R) -> FunctionResult {
    if TypeId::of::<R>() == TypeId::of::<()>() {
        Ok(None)
    } else {
        Ok(Some(Box::new(value)))
    }
}
