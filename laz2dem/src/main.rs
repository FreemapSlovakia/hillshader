mod options;
mod points_reader;
mod progress;
mod rasterization;
mod shared_types;

use std::fs::{exists, remove_file};

use clap::Parser;
use options::{ExistingFileAction, Options};
use points_reader::read;
use rasterization::rasterize;
use shared_types::Job;

fn main() {
    let options = Options::parse();

    let r#continue = exists(&options.output).unwrap()
        && match options.existing_file_action {
            Some(ExistingFileAction::Overwrite) => {
                remove_file(&options.output).unwrap();

                false
            }
            Some(ExistingFileAction::Continue) => true,
            None => panic!("Output file already exitsts. Specify --existing-file-action."),
        };

    let tile_metas = read(&options);

    let mut jobs: Vec<_> = tile_metas.into_iter().map(Job::Rasterize).collect();

    jobs.sort_by_cached_key(|job| job.tile().morton_code());

    rasterize(&options, r#continue, jobs);
}
