# Future Enhancement Ideas for Unveilox CLI

## 1. Configurable Presentation Profiles
- Allow users to define named profiles in a config file (e.g., `~/.config/unveilox/config.toml`) controlling speed, color schemes, and typewriter/TUI defaults.
- Rationale: Reduces repetitive flag usage and enables per-user theming without command-line boilerplate.

## 2. Playlist & Sequencing Mode
- Introduce a `playlist` command that plays multiple poems sequentially with optional interstitial transitions or delays.
- Rationale: Supports performance or reading sessions where curated sequences enhance storytelling.

## 3. Live Library Management
- Add commands to import/export poems at runtime (e.g., `unveilox-cli add path/to/file.txt`) and sync with a user directory.
- Rationale: Makes the CLI useful beyond bundled assets, encouraging personal collections and collaboration.

## 4. Theming & Typography Enhancements
- Provide selectable color palettes and fonts (via TUI attributes) alongside optional background animations or gradients.
- Rationale: Expands the “cinematic” feel and accessibility (e.g., high-contrast themes) for diverse audiences.

## 5. Telemetry-Free Performance Logging
- Offer an opt-in `--profile` mode to measure render times and frame rates, summarizing results after playback.
- Rationale: Helps contributors optimize animations, ensuring smooth experiences on varied hardware.

## 6. Scriptable Hooks & Integrations
- Expose pre/post hooks (shell commands) and simple templates to announce new poems (e.g., send desktop notification).
- Rationale: Enables automation workflows, such as streaming overlays or daily poem reminders.
