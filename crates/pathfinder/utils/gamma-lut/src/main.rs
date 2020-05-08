/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate clap;
extern crate image;

#[macro_use]
extern crate log;

mod gamma_lut;

use clap::{App, Arg};
use gamma_lut::GammaLut;
use image::{DynamicImage, ImageBuffer, Luma};

const CONTRAST: f32 = 0.0;
const GAMMA: f32 = 0.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorU {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ColorU {
    #[inline]
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> ColorU {
        ColorU {
            r: r,
            g: g,
            b: b,
            a: a,
        }
    }
}

pub fn main() {
    let app = App::new("Pathfinder Gamma LUT Generator")
        .version("0.1")
        .author("The Pathfinder Project Developers")
        .about("Generates gamma lookup tables for use with Pathfinder")
        .arg(Arg::with_name("OUTPUT-PATH").help("The `.png` image to produce")
                                          .required(true)
                                          .index(1));
    let matches = app.get_matches();

    let gamma_lut = GammaLut::new(CONTRAST, GAMMA, GAMMA);
    let mut image = ImageBuffer::new(256, gamma_lut.tables.len() as u32);
    for (table_index, table) in gamma_lut.tables.iter().enumerate() {
        for (color_index, &color) in table.iter().enumerate() {
            image.put_pixel(color_index as u32, table_index as u32, Luma([color]))
        }
    }

    let output_path = matches.value_of("OUTPUT-PATH").unwrap();

    DynamicImage::ImageLuma8(image).save(output_path).unwrap();
}
