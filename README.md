# flame

> A volumetric fire simulation with your live OS logo rising through the flames.

A cellular-automaton fire burns upward from the bottom of the screen. The live OS logo (particles) sits in the middle and absorbs heat from the fire below it, causing it to glow and "burn".

## Visual elements

- **Fire grid**. Bottom-up fire simulation with heat values.
- **Sparks**. Rising embers.
- **Logo cells**. The dynamic OS logo made of particles that heat up and glow based on the fire below them.
- **Color palette**. Dynamically generated from your system accent color.

## Dynamic / live behavior

- **Live logo**. Centered logo uses live `logo_text` + kernel from `get_system_info()`.
- **System load reactions**. Higher CPU and memory usage makes the fire burn hotter, taller, and more intense. The logo glows more dramatically under load.
- **Per-machine variation**. `host_bias` gives subtle differences in fire behavior per computer.
- **Accent integration**. Fire colors and logo heating are driven by your current Windows accent.

## Configuration (registry)

Under `HKEY_CURRENT_USER\Software\local76\flame`:

- `FlameHeight`: base height / intensity of the fire.
- `SparkCount`: number of rising sparks.

Global:

- `ColorTheme`, `GlobalScanlines` as usual.

## Notes

- One of the most "alive" feeling scenes — it reacts strongly to what your machine is doing.
- The logo cells have temperature that fades when not heated by fire.
- Works great with scanlines enabled.

Part of the [screensavers](https://github.com/local76/screensavers) collection. See the root README for installation.
