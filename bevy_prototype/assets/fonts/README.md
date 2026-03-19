Place a TTF font named `FiraSans-Bold.ttf` here so the UI text renders.

Options:
- Copy a TTF file to this folder and keep the filename `FiraSans-Bold.ttf`.
- Or edit `src/setup.rs` to point to a different font path you already have.

Example (PowerShell):

Copy-Item -Path "C:\path\to\your\FiraSans-Bold.ttf" -Destination "assets/fonts/FiraSans-Bold.ttf"

After placing the font, run:

cargo run

The UI speed and heading indicators should now appear in the bottom-left.
