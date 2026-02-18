use crate::shared_types::Source;
use dem_common::overview::OverviewResampling as CommonOverviewResampling;
use clap::{ArgGroup, Parser, ValueEnum};
use std::{
    fmt::{Display, Formatter},
    num::ParseIntError,
    path::PathBuf,
    str::FromStr,
};
use tilemath::{bbox::BBox, constants::WEB_MERCATOR_EXTENT};

#[derive(Clone, Debug, Parser, PartialEq)]
#[clap(group = ArgGroup::new("exclusive").required(true))]
pub struct Options {
    /// Output mbtiles file
    pub output: PathBuf,

    /// Source as LAZ tile DB
    #[clap(long, group = "exclusive")]
    pub laz_tile_db: Option<PathBuf>,

    /// Source as LAZ index DB referring *.laz files
    #[clap(long, group = "exclusive")]
    pub laz_index_db: Option<PathBuf>,

    /// EPSG:3857 bounding box to render
    #[clap(long)]
    pub bbox: BBox,

    /// Projection of points if reading from *.laz; default is EPSG:3857
    #[clap(long, conflicts_with = "laz_tile_db")]
    pub source_projection: Option<String>,

    /// Max zoom level of tiles to generate
    #[clap(long)]
    pub zoom_level: u8,

    /// If LAZ tile DB is used then use value of `--zoom-level` argument of `laztile`
    /// If LAZ index is used then use zoom level to determine size of tile to process at once.
    #[clap(long, default_value_t = 16)]
    pub unit_zoom_level: u8,

    /// Tile size
    #[clap(long, default_value_t = 256)]
    pub tile_size: u16,

    /// Buffer size in pixels to prevent artifacts at tieledges
    #[clap(long, default_value_t = 40)]
    pub buffer: u32,

    /// action to take if output file exists
    #[clap(long, value_enum)]
    pub existing_file_action: Option<ExistingFileAction>,

    /// LRU cache size
    #[clap(long, default_value_t = 4096)]
    pub lru_size: usize,

    /// Resampling method for overview DEM tiles
    #[clap(long, value_enum, default_value_t = OverviewResampling::Lanczos)]
    pub overview_resampling: OverviewResampling,
}

impl Options {
    pub fn pixels_per_meter(&self) -> f64 {
        ((u64::from(self.tile_size) << self.zoom_level) as f64) / 2.0 / WEB_MERCATOR_EXTENT
    }

    pub fn source(&self) -> Source {
        self.laz_index_db.clone().map_or_else(
            || {
                self.laz_tile_db
                    .clone()
                    .map_or_else(|| unreachable!("only one"), Source::LazTileDb)
            },
            Source::LazIndexDb,
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum OverviewResampling {
    Lanczos,
    Bilinear,
}

impl From<OverviewResampling> for CommonOverviewResampling {
    fn from(value: OverviewResampling) -> Self {
        match value {
            OverviewResampling::Lanczos => Self::Lanczos,
            OverviewResampling::Bilinear => Self::Bilinear,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum ExistingFileAction {
    Overwrite,
    Continue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rgb(pub image::Rgb<u8>);

impl FromStr for Rgb {
    type Err = ParseIntError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        u32::from_str_radix(string, 16).map(|color| {
            let [_, r, g, b] = color.to_be_bytes();

            Self(image::Rgb([r, g, b]))
        })
    }
}

#[derive(ValueEnum, Debug, Clone, PartialEq, Eq)]
pub enum Format {
    Jpeg,
    Png,
}

impl Display for Format {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}",
            match self {
                Self::Jpeg => "jpeg",
                Self::Png => "png",
            }
        )
    }
}
