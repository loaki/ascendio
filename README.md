# Ascendio

Incremental game: shape a planet, let time run, evolve the animal tree of life. Mobile first (Android + web), written in Rust with macroquad.

```bash
make run      # desktop dev build
make serve    # web build on the LAN
make test     # tests
make lint     # clippy
make release  # Android APK (needs Docker) published as a GitHub release
make upload-key  # once: the Google Play upload key, in .android/ (back it up)
make aab      # Google Play bundle -> target/ascendio.aab
```

The Android build checks the latest GitHub release at launch and shows a banner when a newer one exists.
