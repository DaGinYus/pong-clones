# Pong-Gd

Pong-E in Godot with game logic written in Rust. A project to learn Godot and improve fluency in Rust simultaneously. The reference for this project may be found at https://www.pong-story.com/LAWN_TENNIS.pdf.
All efforts have been made to render this version as faithful to the original as possible. As such, several features exist in this version that likely do not exist in most pong clones:
- Proportions based on the exact timings of the sync signals, translated into pixels
- 7-segment display generation, rather than using rendered fonts
- Original game velocity vectors (the game did not have any physics engine to speak of, so the possible velocities were all discrete)

The code is a bit messy since this is my first experience with Godot. I tried making everything procedurally generated through Rust alone, but this workflow is less convenient. I've learned that leveraging the editor
might be useful for future projects.

To run:
- Install Rust at https://www.rust-lang.org/tools/install
- Download Godot at https://godotengine.org/download/
- Clone/download this project
- Run `cargo build` inside the rust folder
- Add `project.godot` for the folder to Godot project list
- Run!
