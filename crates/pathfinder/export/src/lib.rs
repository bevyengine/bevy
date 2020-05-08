// pathfinder/export/src/lib.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use pathfinder_content::outline::ContourIterFlags;
use pathfinder_content::segment::SegmentKind;
use pathfinder_renderer::scene::Scene;
use pathfinder_geometry::vector::{Vector2F, vec2f};
use std::fmt;
use std::io::{self, Write};

mod pdf;
use pdf::Pdf;

pub enum FileFormat {
    /// Scalable Vector Graphics
    SVG,

    /// Portable Document Format
    PDF,

    /// PostScript
    PS,
}

pub trait Export {
    fn export<W: Write>(&self, writer: &mut W, format: FileFormat) -> io::Result<()>;
}

impl Export for Scene {
    fn export<W: Write>(&self, writer: &mut W, format: FileFormat) -> io::Result<()> {
        match format {
            FileFormat::SVG => export_svg(self, writer),
            FileFormat::PDF => export_pdf(self, writer),
            FileFormat::PS => export_ps(self, writer)
        }
    }
}

fn export_svg<W: Write>(scene: &Scene, writer: &mut W) -> io::Result<()> {
    let view_box = scene.view_box();
    writeln!(
        writer,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\">",
        view_box.origin().x(),
        view_box.origin().y(),
        view_box.size().x(),
        view_box.size().y()
    )?;
    for (paint, outline, name) in scene.paths() {
        write!(writer, "    <path")?;
        if !name.is_empty() {
            write!(writer, " id=\"{}\"", name)?;
        }
        writeln!(writer, " fill=\"{:?}\" d=\"{:?}\" />", paint, outline)?;
    }
    writeln!(writer, "</svg>")?;
    Ok(())
}

fn export_pdf<W: Write>(scene: &Scene, writer: &mut W) -> io::Result<()> {
    let mut pdf = Pdf::new();
    let view_box = scene.view_box();
    pdf.add_page(view_box.size());

    let height = view_box.size().y();
    let tr = |v: Vector2F| -> Vector2F {
        let r = v - view_box.origin();
        vec2f(r.x(), height - r.y())
    };

    for (paint, outline, _) in scene.paths() {
        // TODO(pcwalton): Gradients and patterns.
        if paint.is_color() {
            pdf.set_fill_color(paint.base_color());
        }

        for contour in outline.contours() {
            for (segment_index, segment) in contour.iter(ContourIterFlags::empty()).enumerate() {
                if segment_index == 0 {
                    pdf.move_to(tr(segment.baseline.from()));
                }

                match segment.kind {
                    SegmentKind::None => {}
                    SegmentKind::Line => pdf.line_to(tr(segment.baseline.to())),
                    SegmentKind::Quadratic => {
                        let current = segment.baseline.from();
                        let c = segment.ctrl.from();
                        let p = segment.baseline.to();
                        let c1 = c * (2.0 / 3.0) + current * (1.0 / 3.0);
                        let c2 = c * (2.0 / 3.0) + p * (1.0 / 3.0);
                        pdf.cubic_to(c1, c2, p);
                    }
                    SegmentKind::Cubic => {
                        pdf.cubic_to(tr(segment.ctrl.from()),
                                     tr(segment.ctrl.to()),
                                     tr(segment.baseline.to()))
                    }
                }
            }

            if contour.is_closed() {
                pdf.close();
            }
        }

        // closes implicitly
        pdf.fill();
    }
    pdf.write_to(writer)
}

fn export_ps<W: Write>(scene: &Scene, writer: &mut W) -> io::Result<()> {
    struct P(Vector2F);
    impl fmt::Display for P {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{} {}", self.0.x(), self.0.y())
        }
    }

    let view_box = scene.view_box();
    writeln!(writer, "%!PS-Adobe-3.0 EPSF-3.0")?;
    writeln!(writer, "%%BoundingBox: {:.0} {:.0}",
        P(view_box.origin()),
        P(view_box.size()),
    )?;
    writeln!(writer, "%%HiResBoundingBox: {} {}",
        P(view_box.origin()),
        P(view_box.size()),
    )?;
    writeln!(writer, "0 {} translate", view_box.size().y())?;
    writeln!(writer, "1 -1 scale")?;

    for (paint, outline, name) in scene.paths() {
        if !name.is_empty() {
            writeln!(writer, "newpath % {}", name)?;
        } else {
            writeln!(writer, "newpath")?;
        }

        for contour in outline.contours() {
            for (segment_index, segment) in contour.iter(ContourIterFlags::empty()).enumerate() {
                if segment_index == 0 {
                    writeln!(writer, "{} moveto", P(segment.baseline.from()))?;
                }

                match segment.kind {
                    SegmentKind::None => {}
                    SegmentKind::Line => {
                        writeln!(writer, "{} lineto", P(segment.baseline.to()))?;
                    }
                    SegmentKind::Quadratic => {
                        let current = segment.baseline.from();
                        let c = segment.ctrl.from();
                        let p = segment.baseline.to();
                        let c1 = c * (2.0 / 3.0) + current * (1.0 / 3.0);
                        let c2 = c * (2.0 / 3.0) + p * (1.0 / 3.0);
                        writeln!(writer, "{} {} {} curveto", P(c1), P(c2), P(p))?;
                    }
                    SegmentKind::Cubic => {
                        writeln!(writer, "{} {} {} curveto",
                            P(segment.ctrl.from()),
                            P(segment.ctrl.to()),
                            P(segment.baseline.to())
                        )?;
                    }
                }
            }

            if contour.is_closed() {
                writeln!(writer, "closepath")?;
            }
        }

        // TODO(pcwalton): Gradients and patterns.
        if paint.is_color() {
            let color = paint.base_color();
            writeln!(writer, "{} {} {} setrgbcolor", color.r, color.g, color.b)?;
        }

        writeln!(writer, "fill")?;
    }
    writeln!(writer, "showpage")?;
    Ok(())
}


