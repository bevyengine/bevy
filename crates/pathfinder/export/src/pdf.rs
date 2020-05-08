// pathfinder/export/src/pdf.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! This is a heavily modified version of the pdfpdf crate by Benjamin Kimock <kimockb@gmail.com>
//! (aka. saethlin)

use deflate::Compression;
use pathfinder_color::ColorU;
use pathfinder_geometry::vector::Vector2F;
use std::io::{self, Write};

struct Counter<T> {
    inner: T,
    count: u64
}
impl<T> Counter<T> {
    pub fn new(inner: T) -> Counter<T> {
        Counter {
            inner,
            count: 0
        }
    }
    pub fn pos(&self) -> u64 {
        self.count
    }
}
impl<W: Write> Write for Counter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.inner.write(buf) {
            Ok(n) => {
                self.count += n as u64;
                Ok(n)
            },
            Err(e) => Err(e)
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.inner.write_all(buf)?;
        self.count += buf.len() as u64;
        Ok(())
    }
}

/// Represents a PDF internal object
struct PdfObject {
    contents: Vec<u8>,
    is_page: bool,
    is_xobject: bool,
    offset: Option<u64>,
}

/// The top-level struct that represents a (partially) in-memory PDF file
pub struct Pdf {
    page_buffer: Vec<u8>,
    objects: Vec<PdfObject>,
    page_size: Option<Vector2F>,
    compression: Option<Compression>,
}

impl Default for Pdf {
    fn default() -> Self {
        Self::new()
    }
}

impl Pdf {
    /// Create a new blank PDF document
    #[inline]
    pub fn new() -> Self {
        Self {
            page_buffer: Vec::new(),
            objects: vec![
                PdfObject {
                    contents: Vec::new(),
                    is_page: false,
                    is_xobject: false,
                    offset: None,
                },
                PdfObject {
                    contents: Vec::new(),
                    is_page: false,
                    is_xobject: false,
                    offset: None,
                },
            ],
            page_size: None,
            compression: Some(Compression::Fast)
        }
    }

    fn add_object(&mut self, data: Vec<u8>, is_page: bool, is_xobject: bool) -> usize {
        self.objects.push(PdfObject {
            contents: data,
            is_page,
            is_xobject,
            offset: None,
        });
        self.objects.len()
    }

    /// Set the color for all subsequent drawing operations
    #[inline]
    pub fn set_fill_color(&mut self, color: ColorU) {
        let norm = |color| f32::from(color) / 255.0;
        writeln!(self.page_buffer, "{} {} {} rg",
            norm(color.r),
            norm(color.g),
            norm(color.b)
        ).unwrap();
    }

    /// Move to a new page in the PDF document
    #[inline]
    pub fn add_page(&mut self, size: Vector2F) {
        // Compress and write out the previous page if it exists
        if !self.page_buffer.is_empty() {
            self.end_page();
            self.page_buffer.clear();
        }

        self.page_buffer
            .extend("/DeviceRGB cs /DeviceRGB CS\n1 j 1 J\n".bytes());
        self.page_size = Some(size);
    }

    pub fn move_to(&mut self, p: Vector2F)  {
        writeln!(self.page_buffer, "{} {} m", p.x(), p.y()).unwrap();
    }

    pub fn line_to(&mut self, p: Vector2F) {
        writeln!(self.page_buffer, "{} {} l", p.x(), p.y()).unwrap();
    }

    pub fn cubic_to(&mut self, c1: Vector2F, c2: Vector2F, p: Vector2F) {
        writeln!(self.page_buffer, "{} {} {} {} {} {} c", c1.x(), c1.y(), c2.x(), c2.y(), p.x(), p.y()).unwrap();
    }
    pub fn fill(&mut self) {
        writeln!(self.page_buffer, "f").unwrap();
    }

    pub fn close(&mut self) {
        writeln!(self.page_buffer, "h").unwrap();
    }
    /// Dump a page out to disk
    fn end_page(&mut self) {
        let size = match self.page_size.take() {
            Some(size) => size,
            None => return // no page started
        };
        let page_stream = if let Some(level) = self.compression {
            let compressed = deflate::deflate_bytes_zlib_conf(&self.page_buffer, level);
            let mut page = format!(
                "<< /Length {} /Filter [/FlateDecode] >>\nstream\n",
                compressed.len()
            )
            .into_bytes();
            page.extend_from_slice(&compressed);
            page.extend(b"endstream\n");
            page
        } else {
            let mut page = Vec::new();
            page.extend(format!("<< /Length {} >>\nstream\n", self.page_buffer.len()).bytes());
            page.extend(&self.page_buffer);
            page.extend(b"endstream\n");
            page
        };

        // Create the stream object for this page
        let stream_object_id = self.add_object(page_stream, false, false);

        // Create the page object, which describes settings for the whole page
        let mut page_object = b"<< /Type /Page\n \
            /Parent 2 0 R\n \
            /Resources <<\n"
            .to_vec();

        for (idx, _obj) in self.objects.iter().enumerate().filter(|&(_, o)| o.is_xobject) {
            write!(page_object, "/XObject {} 0 R ", idx+1).unwrap();
        }

        write!(page_object,
            " >>\n \
                /MediaBox [0 0 {} {}]\n \
                /Contents {} 0 R\n\
                >>\n",
            size.x(), size.y(), stream_object_id
        ).unwrap();
        self.add_object(page_object, true, false);
    }

    /// Write the in-memory PDF representation to disk
    pub fn write_to<W>(&mut self, writer: W) -> io::Result<()> where W: Write {
        let mut out = Counter::new(writer);
        out.write_all(b"%PDF-1.7\n%\xB5\xED\xAE\xFB\n")?;

        if !self.page_buffer.is_empty() {
            self.end_page();
        }

        // Write out each object
        for (idx, obj) in self.objects.iter_mut().enumerate().skip(2) {
            obj.offset = Some(out.pos());
            write!(out, "{} 0 obj\n", idx+1)?;
            out.write_all(&obj.contents)?;
            out.write_all(b"endobj\n")?;
        }

        // Write out the page tree object
        self.objects[1].offset = Some(out.pos());
        out.write_all(b"2 0 obj\n")?;
        out.write_all(b"<< /Type /Pages\n")?;
        write!(out,
            "/Count {}\n",
            self.objects.iter().filter(|o| o.is_page).count()
        )?;
        out.write_all(b"/Kids [")?;
        for (idx, _obj) in self.objects.iter().enumerate().filter(|&(_, obj)| obj.is_page) {
            write!(out, "{} 0 R ", idx + 1)?;
        }
        out.write_all(b"] >>\nendobj\n")?;

        // Write out the catalog dictionary object
        self.objects[0].offset = Some(out.pos());
        out.write_all(b"1 0 obj\n<< /Type /Catalog\n/Pages 2 0 R >>\nendobj\n")?;

        // Write the cross-reference table
        let startxref = out.pos() + 1; // NOTE: apparently there's some 1-based indexing??
        out.write_all(b"xref\n")?;
        write!(out, "0 {}\n", self.objects.len() + 1)?;
        out.write_all(b"0000000000 65535 f \n")?;

        for obj in &self.objects {
            write!(out, "{:010} 00000 f \n", obj.offset.unwrap())?;
        }

        // Write the document trailer
        out.write_all(b"trailer\n")?;
        write!(out, "<< /Size {}\n", self.objects.len())?;
        out.write_all(b"/Root 1 0 R >>\n")?;

        // Write the offset to the xref table
        write!(out, "startxref\n{}\n", startxref)?;

        // Write the PDF EOF
        out.write_all(b"%%EOF")?;

        Ok(())
    }
}
