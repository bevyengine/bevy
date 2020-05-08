// pathfinder/lottie/src/lib.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Experimental support for Lottie. This is very incomplete.

use serde::{Deserialize, Serialize};
use serde_json::Error as JSONError;
use std::io::Read;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lottie {
    #[serde(rename = "v")]
    pub version: String,
    #[serde(rename = "fr")]
    pub frame_rate: i64,
    #[serde(rename = "ip")]
    pub in_point: i64,
    #[serde(rename = "op")]
    pub out_point: i64,
    #[serde(rename = "w")]
    pub width: f64,
    #[serde(rename = "h")]
    pub height: f64,
    #[serde(rename = "ddd")]
    pub three_d: i64,
    pub assets: Vec<Asset>,
    pub layers: Vec<Layer>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {}

// FIXME(pcwalton): Using an untagged enum is a botch here. There actually is a tag: it's just an
// integer, which `serde_json` doesn't support natively.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Layer {
    Shape {
        #[serde(rename = "ddd")]
        three_d: i64,
        #[serde(rename = "ind")]
        index: i64,
        #[serde(rename = "nm")]
        name: String,
        #[serde(rename = "ks")]
        transform: Transform,
        #[serde(rename = "ao")]
        auto_orient: i64,
        #[serde(rename = "ip")]
        in_point: i64,
        #[serde(rename = "op")]
        out_point: i64,
        #[serde(rename = "st")]
        start_time: i64,
        #[serde(rename = "bm")]
        blend_mode: i64,
        #[serde(rename = "sr")]
        stretch: i64,
        #[serde(rename = "ln")]
        #[serde(default)]
        layer_id: Option<String>,
        shapes: Vec<Shape>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transform {
    #[serde(rename = "p")]
    pub position: MultidimensionalPropertyValue,
    #[serde(rename = "a")]
    pub anchor_point: MultidimensionalPropertyValue,
    #[serde(rename = "s")]
    pub scale: MultidimensionalPropertyValue,
    #[serde(rename = "r")]
    pub rotation: PropertyValue,
    #[serde(rename = "o")]
    #[serde(default)]
    pub opacity: Option<PropertyValue>,
    #[serde(rename = "px")]
    #[serde(default)]
    pub position_x: Option<PropertyValue>,
    #[serde(rename = "py")]
    #[serde(default)]
    pub position_y: Option<PropertyValue>,
    #[serde(rename = "pz")]
    #[serde(default)]
    pub position_z: Option<PropertyValue>,
    #[serde(rename = "sk")]
    #[serde(default)]
    pub skew: Option<PropertyValue>,
    #[serde(rename = "sa")]
    #[serde(default)]
    pub skew_axis: Option<PropertyValue>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PropertyValue {
    Value {
        #[serde(rename = "k")]
        value: f32,
        #[serde(rename = "x")]
        #[serde(default)]
        expression: Option<String>,
        #[serde(rename = "ix")]
        #[serde(default)]
        index: Option<i64>,
    },
    KeyframedValue {
        #[serde(rename = "k")]
        keyframes: Vec<KeyframeValue>,
        #[serde(rename = "x")]
        #[serde(default)]
        expression: Option<String>,
        #[serde(rename = "ix")]
        #[serde(default)]
        index: Option<i64>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyframeValue {
    #[serde(rename = "s")]
    #[serde(default)]
    pub start: Option<Vec<f32>>,
    #[serde(rename = "t")]
    pub time: i64,
    #[serde(rename = "i")]
    #[serde(default)]
    pub interpolation: Option<OffsetInterpolation>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Interpolation {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OffsetInterpolation {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OffsetKeyframe {
    #[serde(rename = "s")]
    #[serde(default)]
    pub start: Option<Vec<f32>>,
    #[serde(rename = "t")]
    pub time: i64,
    #[serde(rename = "i")]
    #[serde(default)]
    pub in_value: Option<OffsetInterpolation>,
    #[serde(rename = "o")]
    #[serde(default)]
    pub out_value: Option<OffsetInterpolation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MultidimensionalPropertyValue {
    Value {
        #[serde(rename = "k")]
        value: Vec<f32>,
        #[serde(rename = "x")]
        #[serde(default)]
        expression: Option<String>,
        #[serde(rename = "ix")]
        #[serde(default)]
        index: Option<i64>,
    },
    KeyframedValue {
        #[serde(rename = "k")]
        keyframes: Vec<OffsetKeyframe>,
        #[serde(rename = "x")]
        #[serde(default)]
        expression: Option<String>,
        #[serde(rename = "ix")]
        #[serde(default)]
        index: Option<i64>,
        #[serde(rename = "ti")]
        #[serde(default)]
        in_tangent: Option<i64>,
        #[serde(rename = "to")]
        #[serde(default)]
        out_tangent: Option<i64>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "ty")]
pub enum Shape {
    #[serde(rename = "gr")]
    Group {
        #[serde(rename = "it")]
        items: Vec<Shape>,
        #[serde(rename = "nm")]
        name: String,
    },
    #[serde(rename = "sh")]
    Shape {
        #[serde(rename = "ks")]
        vertices: ShapeVertices,
        #[serde(rename = "d")]
        #[serde(default)]
        direction: Option<i64>,
    },
    #[serde(rename = "fl")]
    Fill {
        #[serde(rename = "nm")]
        #[serde(default)]
        name: Option<String>,
        #[serde(rename = "o")]
        #[serde(default)]
        opacity: Option<PropertyValue>,
        #[serde(rename = "c")]
        color: MultidimensionalPropertyValue,
    },
    #[serde(rename = "tr")]
    Transform {
        #[serde(rename = "r")]
        rotation: PropertyValue,
        #[serde(rename = "sk")]
        skew: PropertyValue,
        #[serde(rename = "sa")]
        skew_axis: PropertyValue,
        #[serde(rename = "p")]
        position: MultidimensionalPropertyValue,
        #[serde(rename = "a")]
        anchor_point: MultidimensionalPropertyValue,
        #[serde(rename = "s")]
        scale: MultidimensionalPropertyValue,
    },
    #[serde(other)]
    Unimplemented,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ShapeVertices {
    Shape {
        #[serde(rename = "k")]
        value: ShapeProperty,
        #[serde(rename = "x")]
        #[serde(default)]
        expression: Option<String>,
        #[serde(rename = "ix")]
        #[serde(default)]
        index: Option<i64>,
        #[serde(rename = "a")]
        animated: i64,
    },
    ShapeKeyframed {
        #[serde(rename = "k")]
        value: Vec<ShapeKeyframeProperty>,
        #[serde(rename = "x")]
        #[serde(default)]
        expression: Option<String>,
        #[serde(rename = "ix")]
        #[serde(default)]
        index: Option<i64>,
        #[serde(rename = "a")]
        animated: i64,
        #[serde(rename = "ti")]
        #[serde(default)]
        in_tangent: Vec<i64>,
        #[serde(rename = "to")]
        #[serde(default)]
        out_tangent: Vec<i64>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShapeProperty {
    #[serde(rename = "c")]
    pub closed: bool,
    #[serde(rename = "i")]
    pub in_points: Vec<[f32; 2]>,
    #[serde(rename = "o")]
    pub out_points: Vec<[f32; 2]>,
    #[serde(rename = "v")]
    pub vertices: Vec<[f32; 2]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShapeKeyframeProperty {
    #[serde(rename = "s")]
    #[serde(default)]
    pub start: Vec<Option<ShapeProperty>>,
    #[serde(rename = "t")]
    pub time: i64,
    #[serde(rename = "i")]
    #[serde(default)]
    pub in_value: Option<OffsetInterpolation>,
    #[serde(rename = "o")]
    #[serde(default)]
    pub out_value: Option<OffsetInterpolation>,
}

impl Lottie {
    #[inline]
    pub fn from_reader<R>(reader: R) -> Result<Lottie, JSONError> where R: Read {
        serde_json::from_reader(reader)
    }
}
