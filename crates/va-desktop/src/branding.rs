//! The supplied logo is embedded so branding works from any launch directory.
use std::borrow::Cow;

use gpui::prelude::*;
use gpui::{img, px, AssetSource, Img, SharedString};

const LOGO_PATH: &str = "brand/vibe-assembly.svg";
const LOGO: &[u8] = include_bytes!("../../../assets/vibe-assembly-logo.svg");

pub struct BrandAssets;

impl AssetSource for BrandAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        if path == LOGO_PATH {
            return Ok(Some(Cow::Borrowed(LOGO)));
        }
        Ok(crate::design::icon::asset(path).map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut paths = Vec::new();
        if LOGO_PATH.starts_with(path) {
            paths.push(LOGO_PATH.into());
        }
        paths.extend(crate::design::icon::list(path));
        Ok(paths)
    }
}

pub fn logo(size: f32) -> Img {
    img(LOGO_PATH)
        .size(px(size))
        .flex_shrink_0()
        .rounded(px(4.0))
}
