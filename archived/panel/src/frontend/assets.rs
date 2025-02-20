/*
 * This file is part of Mega.
 *
 * It includes code from Zed/crates/assets, which is licensed under the
 * GNU General Public License (GPL) [Version 3]. See below for details.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

//! This file loads assets from `../../assets`,
//! and most of the resources were straightly taken from zed repository
//! We slightly made some changes to adapt with mega code
//! Copyrights of this part belongs to the authors and zed cooperation

use anyhow::anyhow;
use gpui::{AssetSource, SharedString};
use rust_embed_impl::RustEmbed;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "fonts/**/*"]
#[include = "themes/**/*"]
#[exclude = "themes/src/*"]
#[include = "*.md"]
#[exclude = "*.DS_Store"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow!("Error loading assets from {}", path))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| {
                if p.starts_with(path) {
                    Some(p.into())
                } else {
                    None
                }
            })
            .collect())
    }
}
