use omamusi::{
    artwork::Artwork,
    gallery::{CoverGallery, Rect},
};
use std::path::{Path, PathBuf};
fn art(index: u8) -> Artwork {
    Artwork {
        width: 80,
        height: 80,
        rgba: [index, 255 - index, index / 2, 255].repeat(80 * 80),
    }
}

#[test]
fn identical_artwork_groups_tracks_and_pick_avoids_current() {
    let mut gallery = CoverGallery::new();
    gallery.add("/music/one.flac".into(), art(1));
    gallery.add("/copy/two.flac".into(), art(1));
    gallery.add("/copy/two.flac".into(), art(1));
    assert_eq!(gallery.covers.len(), 1);
    let cover = gallery.covers.values().next().unwrap();
    assert_eq!(cover.tracks.len(), 2);
    assert_eq!(
        cover.pick(Some(Path::new("/music/one.flac"))),
        Some(PathBuf::from("/copy/two.flac"))
    );
}

#[test]
fn thumbnail_cache_is_bounded_while_tracks_remain_grouped() {
    let mut gallery = CoverGallery::new();
    for i in 0..138 {
        gallery.add(format!("/music/{i}.flac").into(), art(i));
    }
    assert_eq!(gallery.covers.len(), 138);
    assert_eq!(
        gallery
            .covers
            .values()
            .filter(|c| c.image.is_some())
            .count(),
        128
    );
    assert!(
        gallery
            .covers
            .values()
            .filter_map(|c| c.image.as_ref())
            .all(|image| image.width <= 320 && image.height <= 320)
    );
}

#[test]
fn rotation_changes_one_cover_and_hover_freezes_the_click_target() {
    let mut gallery = CoverGallery::new();
    for i in 0..7 {
        gallery.add(format!("/music/{i}.mp3").into(), art(i));
    }
    let area = Rect {
        x: 0.0,
        y: 0.0,
        width: 650.0,
        height: 400.0,
    };
    let rectangles = gallery.layout(area);
    assert_eq!(rectangles.len(), 6);
    let before: Vec<_> = gallery.tiles.iter().map(|t| t.key.clone()).collect();
    gallery.tick(6.0);
    assert_eq!(
        before
            .iter()
            .zip(&gallery.tiles)
            .filter(|(a, t)| **a != t.key)
            .count(),
        1
    );
    let center = rectangles[0].center();
    gallery.hover(Some(center), area);
    let key = gallery.tiles[0].key.clone();
    for _ in 0..30 {
        gallery.tick(1.0);
    }
    assert_eq!(gallery.tiles[0].key, key);
    assert!(gallery.hit(center, area).is_some());
}

#[test]
fn gallery_crossfade_reserves_outgoing_cover_and_freezes_dominant_image() {
    let mut gallery = CoverGallery::new();
    gallery.add("/one.mp3".into(), art(1));
    gallery.add("/two.mp3".into(), art(2));
    let area = Rect {
        x: 0.0,
        y: 0.0,
        width: 300.0,
        height: 300.0,
    };
    let rectangle = gallery.layout(area)[0];
    gallery.tick(5.0);
    assert_eq!(gallery.tiles[0].fade, 0.0);
    let previous = gallery.tiles[0].previous.clone().unwrap();
    gallery.hover(Some(rectangle.center()), area);
    assert_eq!(gallery.tiles[0].key, previous);
    assert!(gallery.tiles[0].previous.is_none());
    assert_eq!(gallery.tiles[0].fade, 1.0);
    gallery.hover(None, area);
    gallery.tick(5.0);
    gallery.tick(0.6);
    assert!((gallery.tiles[0].fade - 0.5).abs() < 1e-8);
    gallery.add("/three.mp3".into(), art(3));
    let outgoing = gallery.tiles[0].previous.clone().unwrap();
    gallery.layout(area);
    assert!(gallery.tiles[1..].iter().all(|t| t.key != outgoing));
}

#[test]
fn gallery_layout_is_responsive_and_single_cover_does_not_rotate() {
    let mut gallery = CoverGallery::new();
    let area = Rect {
        x: 0.0,
        y: 0.0,
        width: 600.0,
        height: 400.0,
    };
    assert!(gallery.layout(area).is_empty());
    gallery.add("/one.mp3".into(), art(1));
    let first = gallery.layout(area);
    assert_eq!(first.len(), 1);
    assert!(first[0].width <= 240.0);
    let key = gallery.tiles[0].key.clone();
    gallery.tick(30.0);
    assert_eq!(gallery.tiles[0].key, key);
    assert!(gallery.tiles[0].previous.is_none());
    assert!(
        gallery
            .layout(Rect {
                width: 20.0,
                ..area
            })
            .is_empty()
    );
    gallery.clear();
    assert!(gallery.covers.is_empty());
    assert!(gallery.hovered.is_none());
}
