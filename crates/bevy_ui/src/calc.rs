use crate::Val;

pub struct CalcId(u32);

pub enum CalcOps {
    Add(Val, Val),
    Sub(Val, Val),
    Scale(Val, f32),
    Min(Val, Val),
    Max(Val, Val),
    Clamp(Val, Val),
}
