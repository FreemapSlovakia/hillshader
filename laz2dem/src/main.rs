mod geotiff;
mod lanczos;
mod options;
mod points_reader;
mod progress;
mod rasterization;
mod schema;
mod shared_types;

use std::fs::{exists, remove_file};

use clap::Parser;
use options::{ExistingFileAction, Options};
use points_reader::read;
use rasterization::rasterize;
use shared_types::Job;
use shared_types::Source;

fn main() {
    let options = Options::parse();
    let source = options.source();

    let unit_zoom_level = match source {
        Source::GeoTiff(_) => options.unit_zoom_level.min(options.zoom_level),
        _ => {
            assert!(
                options.unit_zoom_level <= options.zoom_level,
                "--unit-zoom-level must be <= --zoom-level for LAZ sources"
            );
            options.unit_zoom_level
        }
    };

    let r#continue = exists(&options.output).unwrap()
        && match options.existing_file_action {
            Some(ExistingFileAction::Overwrite) => {
                remove_file(&options.output).unwrap();

                false
            }
            Some(ExistingFileAction::Continue) => true,
            None => panic!("Output file already exitsts. Specify --existing-file-action."),
        };

    let bbox = match source {
        Source::GeoTiff(path) => geotiff::derived_bbox(&path),
        _ => options
            .bbox
            .clone()
            .expect("Specify --bbox when using LAZ sources."),
    };

    let tile_metas = read(&options, &bbox, unit_zoom_level);

    let mut jobs: Vec<_> = tile_metas.into_iter().map(Job::Rasterize).collect();

    jobs.sort_by_cached_key(|job| job.tile().morton_code());

    rasterize(&options, &bbox, unit_zoom_level, r#continue, jobs);
}
