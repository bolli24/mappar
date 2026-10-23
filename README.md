# mappar

OpenStreetMap editor for desktop and browsers, written in Rust with egui.

- Loads and edits OSM data through the OSM API (OAuth2 login)
- Renders map data styled by MapCSS; the parser lives in `css/` (`mapcss-validate`)
- Background imagery with local/web caching
- Tag and relation editors, JOSM presets
- Web build via Trunk

Demo: https://bolli24.github.io/mappar/
