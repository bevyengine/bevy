pub struct CalcId(u16);

pub enum CalcOps {
    Add(CalcId, CalcId),
    Sub(CalcId, CalcId),
    Scale(CalcId, f32),
    Min(CalcId, CalcId),
    Max(CalcId, CalcId),
    Clamp(CalcId, CalcId),
}
