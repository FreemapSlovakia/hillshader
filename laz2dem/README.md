Usage:

```
Usage: laz2dem [OPTIONS] --bbox <BBOX> --zoom-level <ZOOM_LEVEL> --shadings <SHADINGS> <--laz-tile-db <LAZ_TILE_DB>|--laz-index-db <LAZ_INDEX_DB>> <OUTPUT>

Arguments:
  <OUTPUT>  Output mbtiles file

Options:
      --laz-tile-db <LAZ_TILE_DB>
          Source as LAZ tile DB
      --laz-index-db <LAZ_INDEX_DB>
          Source as LAZ index DB referring *.laz files
      --bbox <BBOX>
          EPSG:3857 bounding box to render
      --source-projection <SOURCE_PROJECTION>
          Projection of points if reading from *.laz; default is EPSG:3857
      --zoom-level <ZOOM_LEVEL>
          Max zoom level of tiles to generate
      --unit-zoom-level <UNIT_ZOOM_LEVEL>
          If LAZ tile DB is used then use value of `--zoom-level` argument of `laztile` If LAZ index is used then use zoom level to determine size of tile to process at once [default: 16]
      --shadings <SHADINGS>
          Shadings; `+` separated componets of shading. Shading component is <method>,method_param1[,method_param2...].
          ‎
          Methods:
          - `oblique` - params: azimuth in degrees, alitutde in degrees
          - `igor` - params: azimuth in degrees
          - `slope` - params: alitutde in degrees
      --contrast <CONTRAST>
          Increase (> 1.0) or decrease (< 1.0) contrast of the shading. Use value higher than 0.0 [default: 1]
      --brightness <BRIGHTNESS>
          Increase (> 0.0) or decrease (< 0.0) brightness of the shading. Use value between -1.0 and 1.0 [default: 0]
      --z-factor <Z_FACTOR>
          Z-factor [default: 1]
      --tile-size <TILE_SIZE>
          Tile size [default: 256]
      --buffer <BUFFER>
          Buffer size in pixels to prevent artifacts at tieledges [default: 40]
      --format <FORMAT>
          Tile image format. For alpha (transparency) support use `png` [default: jpeg] [possible values: jpeg, png]
      --jpeg-quality <JPEG_QUALITY>
          Quality from 0 to 100 when writing to JPEG [default: 80]
      --background-color <BACKGROUND_COLOR>
          Background color when writing to JPEG as it does not support alpha [default: FFFFFF]
      --existing-file-action <EXISTING_FILE_ACTION>
          [possible values: overwrite, continue]
  -h, --help
          Print help
```

Example:

```sh
cargo run --release -- --unit-zoom-level 16 --laz-tile-db /home/martin/14TB/sk-new-dmr/laztiles.sqlite --bbox 2272998,6204873,2275153,6205973 test.mbtiles --zoom-level 20 --z-factor 5 --shadings igor,5060FF60,135+igor,E0D000B0,315+igor,00000080,135+igor-slope,000000FF --background-color FFFFFF --buffer 50
```
