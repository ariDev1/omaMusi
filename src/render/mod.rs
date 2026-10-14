//! Accelerated and software consumers of the same visual state.
pub mod canvas;
pub mod cpu;
pub mod gl;
use crate::{
    artwork::Artwork,
    gallery::{CoverGallery, Rect},
    visual::{VisualFrame, waveform::WaveformPersistence},
};
pub struct Scene<'a> {
    pub frame: &'a VisualFrame,
    pub wave: &'a WaveformPersistence,
    pub cover: Option<&'a Artwork>,
    pub gallery: &'a mut CoverGallery,
    pub gallery_bounds: Rect,
}
