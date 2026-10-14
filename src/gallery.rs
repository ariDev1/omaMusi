//! Bounded thumbnails, shared artwork groups and stable interactive transitions.
use crate::artwork::Artwork;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Rect {
    pub fn center(self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
    pub fn contains(self, (x, y): (f64, f64)) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}
pub struct GalleryCover {
    pub image: Option<Arc<Artwork>>,
    pub tracks: Vec<PathBuf>,
}
impl GalleryCover {
    pub fn pick(&self, current: Option<&Path>) -> Option<PathBuf> {
        let alternatives: Vec<_> = self
            .tracks
            .iter()
            .filter(|p| Some(p.as_path()) != current)
            .collect();
        if !alternatives.is_empty() {
            Some(alternatives[fastrand::usize(..alternatives.len())].clone())
        } else if !self.tracks.is_empty() {
            Some(self.tracks[fastrand::usize(..self.tracks.len())].clone())
        } else {
            None
        }
    }
}
#[derive(Clone, Debug)]
pub struct GalleryTile {
    pub key: String,
    pub previous: Option<String>,
    pub fade: f64,
}
pub struct CoverGallery {
    pub covers: BTreeMap<String, GalleryCover>,
    pub tiles: Vec<GalleryTile>,
    pub hovered: Option<usize>,
    pub scanning: bool,
    elapsed: f64,
    cached: usize,
    rng: fastrand::Rng,
}
impl Default for CoverGallery {
    fn default() -> Self {
        Self::new()
    }
}
impl CoverGallery {
    pub fn new() -> Self {
        Self {
            covers: BTreeMap::new(),
            tiles: Vec::new(),
            hovered: None,
            scanning: false,
            elapsed: 0.0,
            cached: 0,
            rng: fastrand::Rng::new(),
        }
    }
    pub fn clear(&mut self) {
        self.covers.clear();
        self.tiles.clear();
        self.hovered = None;
        self.scanning = false;
        self.elapsed = 0.0;
        self.cached = 0;
    }
    pub fn add(&mut self, path: PathBuf, image: Artwork) -> bool {
        let Some(image) = image.thumbnail(320) else {
            return false;
        };
        let mut digest = Sha256::new();
        digest.update(format!("{}x{}:", image.width, image.height));
        digest.update(&image.rgba);
        let key = format!("{:x}", digest.finalize());
        let cover = self.covers.entry(key).or_insert_with(|| {
            let thumbnail = if self.cached < 128 {
                self.cached += 1;
                Some(Arc::new(image))
            } else {
                None
            };
            GalleryCover {
                image: thumbnail,
                tracks: Vec::new(),
            }
        });
        if !cover.tracks.contains(&path) {
            cover.tracks.push(path);
        }
        true
    }
    pub fn layout(&mut self, area: Rect) -> Vec<Rect> {
        let keys: Vec<_> = self
            .covers
            .iter()
            .filter(|(_, cover)| cover.image.is_some())
            .map(|(key, _)| key.clone())
            .collect();
        if area.width < 40.0
            || area.height < 40.0
            || !area.width.is_finite()
            || !area.height.is_finite()
            || keys.is_empty()
        {
            self.tiles.clear();
            self.hovered = None;
            return Vec::new();
        }
        let gap = 18.0;
        let columns = (((area.width + gap) / 158.0) as usize).clamp(1, 3);
        let rows = (((area.height + gap) / 158.0) as usize).clamp(1, 2);
        let mut count = keys.len().saturating_sub(1).max(1).min(columns * rows);
        if self.hovered.is_some() && !self.tiles.is_empty() {
            count = count.min(self.tiles.len());
        }
        if self.tiles.len() > count {
            self.tiles.truncate(count);
            self.hovered = None;
        }
        while self.tiles.len() < count {
            let visible: HashSet<_> = self
                .tiles
                .iter()
                .flat_map(|tile| std::iter::once(&tile.key).chain(tile.previous.iter()))
                .collect();
            let candidates: Vec<_> = keys.iter().filter(|key| !visible.contains(key)).collect();
            if candidates.is_empty() {
                break;
            }
            let key = candidates[self.rng.usize(..candidates.len())].clone();
            self.tiles.push(GalleryTile {
                key,
                previous: None,
                fade: 1.0,
            });
        }
        let count = self.tiles.len();
        let columns = columns.min(count);
        if columns == 0 {
            return Vec::new();
        }
        let rows = count.div_ceil(columns);
        let side = ((area.width - (columns - 1) as f64 * gap) / columns as f64)
            .min((area.height - (rows - 1) as f64 * gap) / rows as f64)
            .min(240.0);
        let left = area.center().0 - (columns as f64 * side + (columns - 1) as f64 * gap) / 2.0;
        let top = area.center().1 - (rows as f64 * side + (rows - 1) as f64 * gap) / 2.0;
        (0..count)
            .map(|index| Rect {
                x: left + (index % columns) as f64 * (side + gap),
                y: top + (index / columns) as f64 * (side + gap),
                width: side,
                height: side,
            })
            .collect()
    }
    pub fn hover(&mut self, point: Option<(f64, f64)>, area: Rect) {
        let rectangles = self.layout(area);
        self.hovered = point.and_then(|point| rectangles.iter().position(|r| r.contains(point)));
        if let Some(index) = self.hovered {
            let tile = &mut self.tiles[index];
            if tile.fade < 0.5
                && let Some(previous) = &tile.previous
            {
                tile.key = previous.clone();
            }
            tile.previous = None;
            tile.fade = 1.0;
        }
    }
    pub fn hit(&mut self, point: (f64, f64), area: Rect) -> Option<&GalleryCover> {
        self.hover(Some(point), area);
        self.hovered
            .and_then(|index| self.covers.get(&self.tiles[index].key))
    }
    pub fn tick(&mut self, elapsed: f64) {
        if !elapsed.is_finite() || elapsed < 0.0 {
            return;
        }
        for (index, tile) in self.tiles.iter_mut().enumerate() {
            if Some(index) != self.hovered {
                tile.fade = (tile.fade + elapsed / 1.2).min(1.0);
                if tile.fade >= 1.0 {
                    tile.previous = None;
                }
            }
        }
        self.elapsed += elapsed;
        if self.elapsed < 5.0 {
            return;
        }
        self.elapsed %= 5.0;
        let reserved: HashSet<_> = self
            .tiles
            .iter()
            .flat_map(|tile| std::iter::once(&tile.key).chain(tile.previous.iter()))
            .collect();
        let choices: Vec<_> = self
            .covers
            .iter()
            .filter(|(key, cover)| cover.image.is_some() && !reserved.contains(key))
            .map(|(key, _)| key.clone())
            .collect();
        let slots: Vec<_> = self
            .tiles
            .iter()
            .enumerate()
            .filter(|(index, tile)| Some(*index) != self.hovered && tile.previous.is_none())
            .map(|(index, _)| index)
            .collect();
        if !choices.is_empty() && !slots.is_empty() {
            let index = slots[self.rng.usize(..slots.len())];
            let tile = &mut self.tiles[index];
            tile.previous = Some(tile.key.clone());
            tile.key = choices[self.rng.usize(..choices.len())].clone();
            tile.fade = 0.0;
        }
    }
}
