//! The supplied logo is embedded so branding works from any launch directory.
use std::borrow::Cow;

use gpui::prelude::*;
use gpui::{img, px, AssetSource, Img, SharedString};

const LOGO_PATH: &str = "brand/vibe-assembly.svg";
const LOGO: &[u8] = include_bytes!("../../../assets/vibe-assembly-icon.svg");

pub struct BrandAssets;

impl AssetSource for BrandAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok((path == LOGO_PATH).then_some(Cow::Borrowed(LOGO)))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(if LOGO_PATH.starts_with(path) {
            vec![LOGO_PATH.into()]
        } else {
            Vec::new()
        })
    }
}

pub fn logo(size: f32) -> Img {
    img(LOGO_PATH)
        .size(px(size))
        .flex_shrink_0()
        .rounded(px(4.0))
}
