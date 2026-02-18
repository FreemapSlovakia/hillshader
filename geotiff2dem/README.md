Usage:

```sh
cargo run --release -- \
  --geotiff /path/to/dem_3857.tif \
  --zoom-level 16 \
  --buffer 0 \
  --lru-size 4096 \
  --existing-file-action overwrite \
  output.mbtiles
```

Notes:
- Input GeoTIFF must be in EPSG:3857.
- Rendered extent is derived from the input GeoTIFF georeferenced bounds.
- Output MBTiles tile payload is LERC + zstd, compatible with `laz2dem` output.
- `--buffer` is an extra processing margin in pixels (cropped before tile save).
- `--lru-size` controls overview generation cache size.
