-- One row per player: their best RAD and species count, ranked RAD first.
DROP TABLE IF EXISTS scores;
CREATE TABLE scores (
  -- Random, made by the game on first launch; never sent back to clients.
  player_id  TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  rad        INTEGER NOT NULL,
  species    INTEGER NOT NULL,
  -- The taxon index (src/tree.rs) of the last animal they discovered.
  animal     INTEGER NOT NULL,
  -- Server time (ms) the score last improved: the earlier wins a tie.
  reached    INTEGER NOT NULL,
  -- Server time (ms) of the last accepted submit.
  updated    INTEGER NOT NULL
);
CREATE INDEX by_rank ON scores (rad DESC, species DESC, reached ASC);
