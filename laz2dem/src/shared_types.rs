use spade::{HasPosition, Point2};
use std::{fmt::Debug, path::PathBuf, sync::Mutex};
use tilemath::{bbox::BBox, tile::Tile};

pub struct PointWithHeight {
    pub position: Point2<f64>,
    pub height: f64,
}

impl HasPosition for PointWithHeight {
    type Scalar = f64;

    fn position(&self) -> Point2<f64> {
        self.position
    }
}

pub struct TileMeta {
    pub tile: Tile,
    pub bbox: BBox,
    pub points: Mutex<Vec<PointWithHeight>>,
}

impl Debug for TileMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileMeta")
            .field("tile", &self.tile)
            .field("bbox", &self.bbox)
            .finish()
    }
}

#[derive(Debug)]
pub enum Job {
    Rasterize(TileMeta),
    Overview(Tile),
}

impl Job {
    pub const fn tile(&self) -> Tile {
        match self {
            Self::Rasterize(tile_meta) => tile_meta.tile,
            Self::Overview(tile) => *tile,
        }
    }
}

#[derive(Clone)]
pub enum Source {
    LazTileDb(PathBuf),
    LazIndexDb(PathBuf),
    GeoTiff(PathBuf),
}
