# Hillshader

Collections of tools to create hillshading from set of \*.laz files.

Tools:

- [lazindex](./lazindex) - indexes `*.laz` files for faster querying by `laz2dem`
- [laztile](./laztile) - sorts points from `*.laz` files to tiles suitable for `laztile`
- [laz2dem](./laz2dem) - renders clouds of points to hillshading stored as MBTiles
- [geotiff2dem](./geotiff2dem) - renders GeoTIFF DEM to LERC MBTiles

For creating output of smaller area use `laz2dem` with `lazindex`.

For creating output of big area use `laztile` with `laz2dem`. Once you have output from `laztile` you can use it also for small areas and it will make the processing ~2x faster.
