use bevy_math::{Quat, Vec3};

pub trait Tweening {
    fn tween_linear(self, end: Self, t: f32) -> Self;

    fn tween_cubic_in(self, end: Self, t: f32) -> Self;
    fn tween_cubic_out(self, end: Self, t: f32) -> Self;
    fn tween_cubic_in_out(self, end: Self, t: f32) -> Self;
    fn tween_cubic_out_in(self, end: Self, t: f32) -> Self;

    fn tween_elastic_in(self, end: Self, t: f32) -> Self;
    fn tween_elastic_out(self, end: Self, t: f32) -> Self;
    fn tween_elastic_in_out(self, end: Self, t: f32) -> Self;
    fn tween_elastic_out_in(self, end: Self, t: f32) -> Self;

    fn tween_circ_in(self, end: Self, t: f32) -> Self;
    fn tween_circ_out(self, end: Self, t: f32) -> Self;
    fn tween_circ_in_out(self, end: Self, t: f32) -> Self;
    fn tween_circ_out_in(self, end: Self, t: f32) -> Self;

    fn tween_back_in(self, end: Self, t: f32) -> Self;
    fn tween_back_out(self, end: Self, t: f32) -> Self;
    fn tween_back_in_out(self, end: Self, t: f32) -> Self;
    fn tween_back_out_in(self, end: Self, t: f32) -> Self;

    fn tween_bounce_in(self, end: Self, t: f32) -> Self;
    fn tween_bounce_out(self, end: Self, t: f32) -> Self;
    fn tween_bounce_in_out(self, end: Self, t: f32) -> Self;
    fn tween_bounce_out_in(self, end: Self, t: f32) -> Self;

    fn tween_expo_in(self, end: Self, t: f32) -> Self;
    fn tween_expo_out(self, end: Self, t: f32) -> Self;
    fn tween_expo_in_out(self, end: Self, t: f32) -> Self;
    fn tween_expo_out_in(self, end: Self, t: f32) -> Self;

    fn tween_quad_in(self, end: Self, t: f32) -> Self;
    fn tween_quad_out(self, end: Self, t: f32) -> Self;
    fn tween_quad_in_out(self, end: Self, t: f32) -> Self;
    fn tween_quad_out_in(self, end: Self, t: f32) -> Self;

    fn tween_quart_in(self, end: Self, t: f32) -> Self;
    fn tween_quart_out(self, end: Self, t: f32) -> Self;
    fn tween_quart_in_out(self, end: Self, t: f32) -> Self;
    fn tween_quart_out_in(self, end: Self, t: f32) -> Self;

    fn tween_quint_in(self, end: Self, t: f32) -> Self;
    fn tween_quint_out(self, end: Self, t: f32) -> Self;
    fn tween_quint_in_out(self, end: Self, t: f32) -> Self;
    fn tween_quint_out_in(self, end: Self, t: f32) -> Self;

    fn tween_sine_in(self, end: Self, t: f32) -> Self;
    fn tween_sine_out(self, end: Self, t: f32) -> Self;
    fn tween_sine_in_out(self, end: Self, t: f32) -> Self;
    fn tween_sine_out_in(self, end: Self, t: f32) -> Self;
}

#[derive(Clone, Copy)]
pub enum AnimationType {
    Lerp,
    Slerp,

    CubicIn,
    CubicOut,
    CubicInOut,
    CubicOutIn,

    ElasticIn,
    ElasticOut,
    ElasticInOut,
    ElasticOutIn,

    CircIn,
    CircOut,
    CircInOut,
    CircOutIn,

    BackIn,
    BackOut,
    BackInOut,
    BackOutIn,

    BounceIn,
    BounceOut,
    BounceInOut,
    BounceOutIn,

    ExpoIn,
    ExpoOut,
    ExpoInOut,
    ExpoOutIn,

    QuadIn,
    QuadOut,
    QuadInOut,
    QuadOutIn,

    QuartIn,
    QuartOut,
    QuartInOut,
    QuartOutIn,

    QuintIn,
    QuintOut,
    QuintInOut,
    QuintOutIn,

    SineIn,
    SineOut,
    SineInOut,
    SineOutIn,
}

pub fn tween<T: Tweening>(animation_type: AnimationType, start: T, end: T, t: f32) -> T {
    match animation_type {
        AnimationType::Lerp => start.tween_linear(end, t),
        AnimationType::Slerp => start.tween_linear(end, t), //start.slerp(end, lerper),
        AnimationType::CubicIn => start.tween_cubic_in(end, t),

        AnimationType::CubicOut => start.tween_cubic_out(end, t),
        AnimationType::CubicInOut => start.tween_cubic_in_out(end, t),
        AnimationType::CubicOutIn => start.tween_cubic_in_out(end, t),
        AnimationType::ElasticIn => start.tween_elastic_in(end, t),
        AnimationType::ElasticOut => start.tween_elastic_out(end, t),
        AnimationType::ElasticInOut => start.tween_elastic_in_out(end, t),
        AnimationType::ElasticOutIn => start.tween_elastic_out_in(end, t),
        AnimationType::CircIn => start.tween_circ_in(end, t),
        AnimationType::CircOut => start.tween_circ_out(end, t),
        AnimationType::CircInOut => start.tween_circ_in_out(end, t),
        AnimationType::CircOutIn => start.tween_circ_out_in(end, t),
        AnimationType::BackIn => start.tween_back_in(end, t),
        AnimationType::BackOut => start.tween_back_out(end, t),
        AnimationType::BackInOut => start.tween_back_in_out(end, t),
        AnimationType::BackOutIn => start.tween_back_out_in(end, t),
        AnimationType::BounceIn => start.tween_bounce_in(end, t),
        AnimationType::BounceOut => start.tween_bounce_out(end, t),
        AnimationType::BounceInOut => start.tween_bounce_in_out(end, t),
        AnimationType::BounceOutIn => start.tween_bounce_out_in(end, t),
        AnimationType::ExpoIn => start.tween_expo_in(end, t),
        AnimationType::ExpoOut => start.tween_expo_out(end, t),
        AnimationType::ExpoInOut => start.tween_expo_in_out(end, t),
        AnimationType::ExpoOutIn => start.tween_expo_out_in(end, t),
        AnimationType::QuadIn => start.tween_quad_in(end, t),
        AnimationType::QuadOut => start.tween_quad_out(end, t),
        AnimationType::QuadInOut => start.tween_quad_in_out(end, t),
        AnimationType::QuadOutIn => start.tween_quad_out_in(end, t),
        AnimationType::QuartIn => start.tween_quart_in(end, t),
        AnimationType::QuartOut => start.tween_quart_out(end, t),
        AnimationType::QuartInOut => start.tween_quart_in_out(end, t),
        AnimationType::QuartOutIn => start.tween_quart_out_in(end, t),
        AnimationType::QuintIn => start.tween_quint_in(end, t),
        AnimationType::QuintOut => start.tween_quint_out(end, t),
        AnimationType::QuintInOut => start.tween_quint_in_out(end, t),
        AnimationType::QuintOutIn => start.tween_quint_out_in(end, t),
        AnimationType::SineIn => start.tween_sine_in(end, t),
        AnimationType::SineOut => start.tween_sine_out(end, t),
        AnimationType::SineInOut => start.tween_sine_in_out(end, t),
        AnimationType::SineOutIn => start.tween_sine_out_in(end, t),
    }
}

impl Tweening for f32 {
    fn tween_linear(self, end: Self, s: f32) -> Self {
        self + ((end - self) * s)
    }
    fn tween_cubic_in(self, end: Self, s: f32) -> Self {
        let s = cubic_in(s, 0.0, 1.0, 1.0);
        // TODO: do you really need to call linear once again?
        // probably for f32 yes, then in Quat it should be slerp?
        self.tween_linear(end, s)
    }
    fn tween_cubic_out(self, end: Self, s: f32) -> Self {
        let s = cubic_out(s, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }
    fn tween_cubic_in_out(self, end: Self, s: f32) -> Self {
        let s = cubic_in_out(s, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }
    fn tween_cubic_out_in(self, end: Self, s: f32) -> Self {
        let s = cubic_out_in(s, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_elastic_in(self, end: Self, t: f32) -> Self {
        let s = elastic_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_elastic_out(self, end: Self, t: f32) -> Self {
        let s = elastic_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_elastic_in_out(self, end: Self, t: f32) -> Self {
        let s = elastic_in_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_elastic_out_in(self, end: Self, t: f32) -> Self {
        let s = elastic_out_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_circ_in(self, end: Self, t: f32) -> Self {
        let s = circ_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_circ_out(self, end: Self, t: f32) -> Self {
        let s = circ_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_circ_in_out(self, end: Self, t: f32) -> Self {
        let s = circ_in_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_circ_out_in(self, end: Self, t: f32) -> Self {
        let s = circ_out_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_back_in(self, end: Self, t: f32) -> Self {
        let s = back_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_back_out(self, end: Self, t: f32) -> Self {
        let s = back_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_back_in_out(self, end: Self, t: f32) -> Self {
        let s = back_in_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_back_out_in(self, end: Self, t: f32) -> Self {
        let s = back_out_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_bounce_in(self, end: Self, t: f32) -> Self {
        let s = bounce_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_bounce_out(self, end: Self, t: f32) -> Self {
        let s = bounce_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_bounce_in_out(self, end: Self, t: f32) -> Self {
        let s = bounce_in_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_bounce_out_in(self, end: Self, t: f32) -> Self {
        let s = bounce_out_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_expo_in(self, end: Self, t: f32) -> Self {
        let s = expo_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_expo_out(self, end: Self, t: f32) -> Self {
        let s = expo_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_expo_in_out(self, end: Self, t: f32) -> Self {
        let s = expo_in_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_expo_out_in(self, end: Self, t: f32) -> Self {
        let s = expo_out_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_quad_in(self, end: Self, t: f32) -> Self {
        let s = quad_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_quad_out(self, end: Self, t: f32) -> Self {
        let s = quad_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_quad_in_out(self, end: Self, t: f32) -> Self {
        let s = quad_in_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_quad_out_in(self, end: Self, t: f32) -> Self {
        let s = quad_out_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_quart_in(self, end: Self, t: f32) -> Self {
        let s = quart_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_quart_out(self, end: Self, t: f32) -> Self {
        let s = quart_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_quart_in_out(self, end: Self, t: f32) -> Self {
        let s = quart_in_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_quart_out_in(self, end: Self, t: f32) -> Self {
        let s = quart_out_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_quint_in(self, end: Self, t: f32) -> Self {
        let s = quint_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_quint_out(self, end: Self, t: f32) -> Self {
        let s = quint_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_quint_in_out(self, end: Self, t: f32) -> Self {
        let s = quint_in_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_quint_out_in(self, end: Self, t: f32) -> Self {
        let s = quint_out_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_sine_in(self, end: Self, t: f32) -> Self {
        let s = sine_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_sine_out(self, end: Self, t: f32) -> Self {
        let s = sine_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_sine_in_out(self, end: Self, t: f32) -> Self {
        let s = sine_in_out(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }

    fn tween_sine_out_in(self, end: Self, t: f32) -> Self {
        let s = sine_out_in(t, 0.0, 1.0, 1.0);
        self.tween_linear(end, s)
    }
}

impl Tweening for Vec3 {
    fn tween_linear(self, end: Self, t: f32) -> Self {
        self.lerp(end, t)
    }
    fn tween_cubic_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, cubic_in_t(t))
    }
    fn tween_cubic_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, cubic_out_t(t))
    }
    fn tween_cubic_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, cubic_in_out_t(t))
    }
    fn tween_cubic_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, cubic_out_in_t(t))
    }

    fn tween_elastic_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, elastic_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_elastic_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, elastic_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_elastic_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, elastic_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_elastic_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, elastic_out_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_circ_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, circ_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_circ_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, circ_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_circ_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, circ_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_circ_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, circ_out_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_back_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, back_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_back_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, back_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_back_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, back_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_back_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, back_out_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_bounce_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, bounce_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_bounce_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, bounce_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_bounce_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, bounce_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_bounce_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, bounce_out_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_expo_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, expo_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_expo_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, expo_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_expo_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, expo_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_expo_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, expo_out_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_quad_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, quad_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_quad_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, quad_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_quad_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, quad_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_quad_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, quad_out_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_quart_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, quart_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_quart_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, quart_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_quart_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, quart_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_quart_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, quart_out_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_quint_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, quint_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_quint_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, quint_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_quint_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, quint_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_quint_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, quint_out_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_sine_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, sine_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_sine_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, sine_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_sine_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, sine_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_sine_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, sine_out_in(t, 0.0, 1.0, 1.0))
    }
}

impl Tweening for Quat {
    fn tween_linear(self, end: Self, t: f32) -> Self {
        self.lerp(end, t)
    }
    fn tween_cubic_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, cubic_in_t(t))
    }
    fn tween_cubic_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, cubic_out_t(t))
    }
    fn tween_cubic_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, cubic_in_out_t(t))
    }
    fn tween_cubic_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, cubic_out_in_t(t))
    }

    fn tween_elastic_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, elastic_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_elastic_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, elastic_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_elastic_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, elastic_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_elastic_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, elastic_out_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_circ_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, circ_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_circ_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, circ_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_circ_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, circ_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_circ_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, circ_out_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_back_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, back_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_back_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, back_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_back_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, back_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_back_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, back_out_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_bounce_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, bounce_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_bounce_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, bounce_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_bounce_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, bounce_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_bounce_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, bounce_out_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_expo_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, expo_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_expo_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, expo_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_expo_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, expo_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_expo_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, expo_out_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_quad_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, quad_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_quad_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, quad_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_quad_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, quad_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_quad_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, quad_out_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_quart_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, quart_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_quart_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, quart_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_quart_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, quart_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_quart_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, quart_out_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_quint_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, quint_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_quint_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, quint_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_quint_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, quint_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_quint_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, quint_out_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_sine_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, sine_in(t, 0.0, 1.0, 1.0))
    }

    fn tween_sine_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, sine_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_sine_in_out(self, end: Self, t: f32) -> Self {
        self.lerp(end, sine_in_out(t, 0.0, 1.0, 1.0))
    }

    fn tween_sine_out_in(self, end: Self, t: f32) -> Self {
        self.lerp(end, sine_out_in(t, 0.0, 1.0, 1.0))
    }
}

// time, initial, delta, duration
// initial(b) = 0.0
// delta(c) = 1.0
// duration(d) = 1.0 ?
pub fn linear(t: f32, b: f32, c: f32, d: f32) -> f32 {
    return c * t / d + b;
}

// CUBIC

#[inline]
pub fn cubic_in_t(t: f32) -> f32 {
    return t * t * t;
}

#[inline]
pub fn cubic_out_t(t: f32) -> f32 {
    let t = t - 1.0;
    return t * t * t + 1.0;
}
#[inline]
pub fn cubic_in_out_t(t: f32) -> f32 {
    let t = t / (1.0 / 2.0);
    if t < 1.0 {
        return 1.0 / 2.0 * t * t * t;
    }

    let t = t - 2.0;
    return 1.0 / 2.0 * (t * t * t + 2.0) + 0.0;
}
#[inline]
pub fn cubic_out_in_t(t: f32) -> f32 {
    if t < 1.0 / 2.0 {
        return cubic_out(t * 2.0, 0.0, 1.0 / 2.0, 1.0);
    }
    return cubic_in(t * 2.0, 1.0 / 2.0, 1.0 / 2.0, 1.0);
}

#[inline]
pub fn cubic_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    let t = t / d;
    return c * t * t * t + b;
}
#[inline]
pub fn cubic_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    let t = t / d - 1.0;
    return c * (t * t * t + 1.0) + b;
}
#[inline]
pub fn cubic_in_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    let t = t / (d / 2.0);
    if t < 1.0 {
        return c / 2.0 * t * t * t + b;
    }

    let t = t - 2.0;
    return c / 2.0 * (t * t * t + 2.0) + b;
}
#[inline]
pub fn cubic_out_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t < d / 2.0 {
        return cubic_out(t * 2.0, b, c / 2.0, d);
    }
    return cubic_in(t * 2.0 - d, b + c / 2.0, c / 2.0, d);
}

// ELASTIC

#[inline]
pub fn elastic_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t == 0.0 {
        return b;
    }

    let mut t = t / d;
    if t == 1.0 {
        return b + c;
    }

    t -= 1.0;
    let p: f32 = d * 0.3;
    let a: f32 = c * f32::powf(2.0, 10.0 * t);
    let s: f32 = p / 4.0;

    return -(a * f32::sin((t * d - s) * (2.0 * std::f32::consts::PI) / p)) + b;
}

#[inline]
pub fn elastic_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t == 0.0 {
        return b;
    }

    let t: f32 = t / d;
    if t == 1.0 {
        return b + c;
    }

    let p: f32 = d * 0.3;
    let s: f32 = p / 4.0;

    return c
        * f32::powf(2.0, -10.0 * t)
        * f32::sin((t * d - s) * (2.0 * std::f32::consts::PI) / p)
        + c
        + b;
}

#[inline]
pub fn elastic_in_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t == 0.0 {
        return b;
    }
    let mut t = t / d / 2.0;
    if t == 2.0 {
        return b + c;
    }

    let p: f32 = d * (0.3 * 1.5);
    let mut a: f32 = c;
    let s: f32 = p / 4.0;

    if t < 1.0 {
        t -= 1.0;
        a *= f32::powf(2.0, 10.0 * t);
        return -0.5 * (a * f32::sin((t * d - s) * (2.0 * std::f32::consts::PI) / p)) + b;
    }

    t -= 1.0;
    a *= 2_f32.powf(-10.0 * t);
    return a * f32::sin((t * d - s) * (2.0 * std::f32::consts::PI) / p) * 0.5 + c + b;
}

#[inline]
pub fn elastic_out_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t < d / 2.0 {
        return elastic_out(t * 2.0, b, c / 2.0, d);
    }
    return elastic_in(t * 2.0 - d, b + c / 2.0, c / 2.0, d);
}

// CIRC

#[inline]
pub fn circ_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    let t: f32 = t / d;
    return -c * (f32::sqrt(1.0 - t * t) - 1.0) + b;
}

#[inline]
pub fn circ_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    let t: f32 = t / d - 1.0;
    return c * f32::sqrt(1.0 - t * t) + b;
}

#[inline]
pub fn circ_in_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    let t: f32 = t / (d / 2.0);
    if t < 1.0 {
        return -c / 2.0 * (f32::sqrt(1.0 - t * t) - 1.0) + b;
    }

    let t: f32 = t - 2.0;
    return c / 2.0 * (f32::sqrt(1.0 - t * t) + 1.0) + b;
}

#[inline]
pub fn circ_out_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t < d / 2.0 {
        return circ_out(t * 2.0, b, c / 2.0, d);
    }
    return circ_in(t * 2.0 - d, b + c / 2.0, c / 2.0, d);
}

// BACK

#[inline]
pub fn back_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    let s: f32 = 1.70158;
    let t: f32 = t / d;

    return c * t * t * ((s + 1.0) * t - s) + b;
}

#[inline]
pub fn back_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    let s: f32 = 1.70158;
    let t: f32 = t / d - 1.0;

    return c * (t * t * ((s + 1.0) * t + s) + 1.0) + b;
}

#[inline]
pub fn back_in_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    let s: f32 = 1.70158 * 1.525;
    let t: f32 = t / (d / 2.0);

    if t < 1.0 {
        return c / 2.0 * (t * t * ((s + 1.0) * t - s)) + b;
    }

    let t: f32 = t - 2.0;
    return c / 2.0 * (t * t * ((s + 1.0) * t + s) + 2.0) + b;
}

#[inline]
pub fn back_out_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t < d / 2.0 {
        return back_out(t * 2.0, b, c / 2.0, d);
    }
    return back_in(t * 2.0 - d, b + c / 2.0, c / 2.0, d);
}

// BOUNCE

#[inline]
pub fn bounce_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    let mut t: f32 = t / d;

    if t < (1.0 / 2.75) {
        return c * (7.5625 * t * t) + b;
    }

    if t < (2.0 / 2.75) {
        t -= 1.5 / 2.75;
        return c * (7.5625 * t * t + 0.75) + b;
    }

    if t < (2.5 / 2.75) {
        t -= 2.25 / 2.75;
        return c * (7.5625 * t * t + 0.9375) + b;
    }

    t -= 2.625 / 2.75;
    return c * (7.5625 * t * t + 0.984375) + b;
}

#[inline]
pub fn bounce_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    return c - bounce_out(d - t, 0.0, c, d) + b;
}

#[inline]
pub fn bounce_in_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t < d / 2.0 {
        return bounce_in(t * 2.0, b, c / 2.0, d);
    }
    return bounce_out(t * 2.0 - d, b + c / 2.0, c / 2.0, d);
}

#[inline]
pub fn bounce_out_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t < d / 2.0 {
        return bounce_out(t * 2.0, b, c / 2.0, d);
    }
    return bounce_in(t * 2.0 - d, b + c / 2.0, c / 2.0, d);
}

// EXPO

#[inline]
pub fn expo_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t == 0.0 {
        return b;
    }
    return c * f32::powf(2.0, 10.0 * (t / d - 1.0)) + b - c * 0.001;
}

#[inline]
pub fn expo_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t == d {
        return b + c;
    }
    return c * 1.001 * (f32::powf(-2.0, -10.0 * t / d) + 1.0) + b;
}

#[inline]
pub fn expo_in_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t == 0.0 {
        return b;
    }

    if t == d {
        return b + c;
    }

    let t: f32 = t / d * 2.0;

    if t < 1.0 {
        return c / 2.0 * f32::powf(2.0, 10.0 * (t - 1.0)) + b - c * 0.0005;
    }
    return c / 2.0 * 1.0005 * (f32::powf(-2.0, -10.0 * (t - 1.0)) + 2.0) + b;
}

#[inline]
pub fn expo_out_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t < d / 2.0 {
        return expo_out(t * 2.0, b, c / 2.0, d);
    }
    return expo_in(t * 2.0 - d, b + c / 2.0, c / 2.0, d);
}

// QUAD

#[inline]
pub fn quad_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    return c * f32::powi(t / d, 2) + b;
}

#[inline]
pub fn quad_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    let t: f32 = t / d;
    return -c * t * (t - 2.0) + b;
}

#[inline]
pub fn quad_in_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    let t: f32 = t / d * 2.0;

    if t < 1.0 {
        return c / 2.0 * f32::powi(t, 2) + b;
    }
    return -c / 2.0 * ((t - 1.0) * (t - 3.0) - 1.0) + b;
}

#[inline]
pub fn quad_out_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t < d / 2.0 {
        return quad_out(t * 2.0, b, c / 2.0, d);
    }
    return quad_in(t * 2.0 - d, b + c / 2.0, c / 2.0, d);
}

// QUART

#[inline]
pub fn quart_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    return c * f32::powi(t / d, 4) + b;
}

#[inline]
pub fn quart_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    return -c * (f32::powi(t / d - 1.0, 4) - 1.0) + b;
}

#[inline]
pub fn quart_in_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    let t: f32 = t / d * 2.0;

    if t < 1.0 {
        return c / 2.0 * f32::powi(t, 4) + b;
    }
    return -c / 2.0 * (f32::powi(t - 2.0, 4) - 2.0) + b;
}

#[inline]
pub fn quart_out_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t < d / 2.0 {
        return quart_out(t * 2.0, b, c / 2.0, d);
    }
    return quart_in(t * 2.0 - d, b + c / 2.0, c / 2.0, d);
}

// QUINT

#[inline]
pub fn quint_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    return c * f32::powi(t / d, 5) + b;
}

#[inline]
pub fn quint_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    return c * (f32::powi(t / d - 1.0, 5) + 1.0) + b;
}

#[inline]
pub fn quint_in_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    let t: f32 = t / d * 2.0;

    if t < 1.0 {
        return c / 2.0 * f32::powi(t, 5) + b;
    }
    return c / 2.0 * (f32::powi(t - 2.0, 5) + 2.0) + b;
}

#[inline]
pub fn quint_out_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t < d / 2.0 {
        return quint_out(t * 2.0, b, c / 2.0, d);
    }
    return quint_in(t * 2.0 - d, b + c / 2.0, c / 2.0, d);
}

// SINE

#[inline]
pub fn sine_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    return -c * f32::cos(t / d * (std::f32::consts::PI / 2.0)) + c + b;
}

#[inline]
pub fn sine_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    return c * f32::sin(t / d * (std::f32::consts::PI / 2.0)) + b;
}

#[inline]
pub fn sine_in_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    return -c / 2.0 * (f32::cos(std::f32::consts::PI * t / d) - 1.0) + b;
}

#[inline]
pub fn sine_out_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if t < d / 2.0 {
        return sine_out(t * 2.0, b, c / 2.0, d);
    }
    return sine_in(t * 2.0 - d, b + c / 2.0, c / 2.0, d);
}
