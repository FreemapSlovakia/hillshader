use rusqlite::{Connection, Error};

pub fn create_schema(conn: &Connection, meta: &[(&str, &str)]) -> Result<(), Error> {
    conn.execute(
        "CREATE TABLE metadata (
          name TEXT NOT NULL,
          value TEXT NOT NULL,
          UNIQUE(name)
      )",
        (),
    )?;

    conn.execute(
        "CREATE TABLE tiles (
          zoom_level INTEGER NOT NULL,
          tile_column INTEGER NOT NULL,
          tile_row INTEGER NOT NULL,
          tile_data BLOB NOT NULL
        )",
        (),
    )?;

    conn.execute(
        "CREATE UNIQUE INDEX idx_tiles ON tiles (zoom_level, tile_column, tile_row)",
        (),
    )?;

    let mut stmt = conn.prepare("INSERT INTO metadata VALUES (?1, ?2)")?;

    for item in meta {
        stmt.execute(*item)?;
    }

    Ok(())
}
