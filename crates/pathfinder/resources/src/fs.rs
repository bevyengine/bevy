// pathfinder/resources/src/fs.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Reads resources from the filesystem.

use crate::ResourceLoader;
use std::env;
use std::fs::File;
use std::io::{Error as IOError, Read};
use std::path::PathBuf;

pub struct FilesystemResourceLoader {
    pub directory: PathBuf,
}

impl FilesystemResourceLoader {
    pub fn locate() -> FilesystemResourceLoader {
        let mut parent_directory = env::current_dir().unwrap();
        loop {
            // So ugly :(
            let mut resources_directory = parent_directory.clone();
            resources_directory.push("resources");
            if resources_directory.is_dir() {
                let mut shaders_directory = resources_directory.clone();
                let mut textures_directory = resources_directory.clone();
                shaders_directory.push("shaders");
                textures_directory.push("textures");
                if shaders_directory.is_dir() && textures_directory.is_dir() {
                    return FilesystemResourceLoader {
                        directory: resources_directory,
                    };
                }
            }

            if !parent_directory.pop() {
                break;
            }
        }

        panic!("No suitable `resources/` directory found!");
    }
}

impl ResourceLoader for FilesystemResourceLoader {
    fn slurp(&self, virtual_path: &str) -> Result<Vec<u8>, IOError> {
        let mut path = self.directory.clone();
        virtual_path
            .split('/')
            .for_each(|segment| path.push(segment));

        let mut data = vec![];
        File::open(&path)?.read_to_end(&mut data)?;
        Ok(data)
    }
}
