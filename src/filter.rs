use serde::Deserialize;
use std::{fs::File, io::Read, path::Path};
use tracing::error;

use crate::{error::AppError, tweet::Tweet};
use mlua::prelude::*;

#[derive(Debug)]
pub struct Filter {
    pub src: String,
    pub meta: FilterMeta,
}

#[derive(Debug, Deserialize)]
pub struct FilterMeta {
    name: String,
    description: String,
    author: String,
    entrypoint: String,
}

impl Filter {
    pub fn load(dir: &Path) -> Result<Vec<Filter>, AppError> {
        if !dir.is_dir() {
            return Err(AppError::FilterPathNotDir(dir.to_owned()));
        }

        dir.read_dir()?
            .filter_map(|entry| match entry {
                Ok(entry) => Some(entry.path()),
                _ => None,
            })
            .filter(|path| path.is_dir())
            .map(|dir| match Self::load_single(&dir) {
                Ok(filter) => Ok(filter),
                Err(err) => {
                    error!("could not load filter in {}/ : {}", dir.display(), err);
                    Err(err)
                }
            })
            .collect()
    }

    fn load_single(dir: &Path) -> Result<Filter, AppError> {
        if !dir.is_dir() {
            return Err(AppError::FilterPathNotDir(dir.to_owned()));
        }

        let meta_path = dir.join("binchotan.toml");
        let mut meta_buf = String::new();
        File::open(&meta_path)?.read_to_string(&mut meta_buf)?;
        let meta: FilterMeta = toml::from_str(&meta_buf).map_err(AppError::FilterMetaParse)?;

        let mut src = String::new();
        File::open(&dir.join(&meta.entrypoint))?.read_to_string(&mut src)?;

        Ok(Filter { src, meta })
    }

    /// Runs the filter on the given post. The filter is a Lua script which returns a Tweet or null.
    pub fn run(&self, tweet: &Tweet) -> Result<Option<Tweet>, AppError> {
        let lua = Lua::new();
        lua.globals().set("post", lua.to_value(tweet)?)?;
        let ret = lua.load(&self.src).eval()?;
        let v: Option<Tweet> = lua.from_value(ret)?;
        Ok(v)
    }
}