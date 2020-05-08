// pathfinder/utils/svg-to-skia/src/main.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::env;
use usvg::{Node, NodeKind, Options, Paint, PathSegment, Tree};

fn main() {
    let input_path = env::args().skip(1).next().unwrap();
    let tree = Tree::from_file(&input_path, &Options::default()).unwrap();

    println!("#ifndef PAINT_H");
    println!("#define PAINT_H");
    println!("static void paint(SkCanvas *canvas) {{");
    println!("    SkPaint paint;");
    println!("    SkPath path;");
    println!("    paint.setAntiAlias(true);");
    println!("    canvas->clear(SK_ColorWHITE);");

    let root = &tree.root();
    match *root.borrow() {
        NodeKind::Svg(_) => {
            for kid in root.children() {
                process_node(&kid);
            }
        }
        _ => unreachable!(),
    }

    println!("}}");
    println!("#endif");
}

fn process_node(node: &Node) {
    match *node.borrow() {
        NodeKind::Group(_) => {
            for kid in node.children() {
                process_node(&kid)
            }
        }
        NodeKind::Path(ref path) => {
            for segment in path.data.iter() {
                match segment {
                    PathSegment::MoveTo { x, y } => println!("    path.moveTo({}, {});", x, y),
                    PathSegment::LineTo { x, y } => println!("    path.lineTo({}, {});", x, y),
                    PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                        println!("    path.cubicTo({}, {}, {}, {}, {}, {});",
                                 x1, y1, x2, y2, x, y);
                    }
                    PathSegment::ClosePath => println!("    path.close();"),
                }
            }

            if let Some(ref fill) = path.fill {
                set_color(&fill.paint);
                println!("    paint.setStyle(SkPaint::kFill_Style);");
                println!("    canvas->drawPath(path, paint);");
            }

            if let Some(ref stroke) = path.stroke {
                set_color(&stroke.paint);
                println!("    paint.setStrokeWidth({});", stroke.width.value());
                println!("    paint.setStyle(SkPaint::kStroke_Style);");
                println!("    canvas->drawPath(path, paint);");
            }

            println!("    path.reset();");
        }
        _ => {}
    }
}

fn set_color(paint: &Paint) {
    if let Paint::Color(color) = *paint {
        println!("    paint.setColor(0x{:x});",
                ((color.red as u32) << 16) |
                    ((color.green as u32) << 8) |
                    ((color.blue as u32) << 0) |
                    (0xff << 24));
    }
}
