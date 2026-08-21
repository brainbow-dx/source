//! Bundled toolbar icon assets. Real source from [Lucide](https://lucide.dev) (`lucide-static`,
//! ISC-licensed, see each `.svg` file's own `@license` comment header, kept intact alongside its
//! rasterized `.png` twin), fetched directly rather than approximated from memory. Lucide is
//! ISC-licensed, community-governed (a continuation of Feather Icons, created specifically so a
//! popular open-source icon set couldn't stall or close off), and ubiquitous in developer tools.
//!
//! Embedded as pre-rasterized PNGs, not the raw `.svg` bytes, despite `NSImage` nominally
//! supporting SVG data. This was confirmed live as a real, silent failure: `NSImage::initWithData`
//! on this SVG data returned `None` (not a garbled/blank image, a hard `None`), so `NodeKind::
//! Button`'s spawn code took its "icon name not recognized" fallback path and rendered the
//! button's plain-text label instead. That was invisible for glyphs like the chevrons (plain
//! characters, already monochrome), but glaring for the pin button (a full-color 📌 emoji suddenly
//! appearing in an otherwise all-gray toolbar). `NSImage` decodes PNG data without question, so
//! each `.svg` here has a same-named `.png` twin (`rsvg-convert -w 48 -h 48 <name>.svg -o
//! <name>.png`; 48px is 3x a 16pt toolbar icon, comfortable headroom for Retina) checked in
//! alongside it; the `.svg` stays for provenance/re-rasterizing if an icon ever needs to change,
//! but nothing in this crate reads it directly. Each source `.svg`'s `stroke="currentColor"` was
//! changed to `stroke="black"` before rasterizing (a straightforward derivative edit ISC
//! explicitly permits). `NodeKind::Button` loads the PNG as a template `NSImage`
//! (`setTemplate(true)`), which macOS re-tints from a plain black-on-transparent source.
//!
//! Looked up by a portable, symbolic name (`escher_core::element::Button::icon`). A surface that
//! doesn't know how to render icons at all just uses the button's plain-text `label` instead, so
//! this is additive, never a hard requirement for `escher_chalk::toolbar` to keep working
//! everywhere else it already renders (terminal, web, Bevy).

const MENU: &[u8] = include_bytes!("../assets/icons/menu.png");
const CHEVRON_LEFT: &[u8] = include_bytes!("../assets/icons/chevron-left.png");
const CHEVRON_RIGHT: &[u8] = include_bytes!("../assets/icons/chevron-right.png");
const REFRESH_CW: &[u8] = include_bytes!("../assets/icons/refresh-cw.png");
const PIN: &[u8] = include_bytes!("../assets/icons/pin.png");

/// The raw (PNG-encoded) image bytes for `name`, if this crate bundles one. See this module's
/// own doc comment for where they came from. `None` for anything not in the small, fixed set the
/// toolbar actually uses today; a caller falls back to plain text in that case, same as an
/// icon-unaware surface would.
pub fn icon_bytes(name: &str) -> Option<&'static [u8]> {
    match name {
        "menu" => Some(MENU),
        "chevron-left" => Some(CHEVRON_LEFT),
        "chevron-right" => Some(CHEVRON_RIGHT),
        "refresh-cw" => Some(REFRESH_CW),
        "pin" => Some(PIN),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use objc2::AnyThread;
    use objc2_app_kit::NSImage;
    use objc2_foundation::NSData;

    #[test]
    fn every_bundled_icon_decodes_via_nsimage() {
        for name in ["menu", "chevron-left", "chevron-right", "refresh-cw", "pin"] {
            let bytes = super::icon_bytes(name).unwrap();
            let data = NSData::with_bytes(bytes);
            let image = NSImage::initWithData(NSImage::alloc(), &data);
            assert!(image.is_some(), "{name} failed to decode ({} bytes)", bytes.len());
            let image = image.unwrap();
            let size = image.size();
            println!("{name}: decoded, size={:?}", size);
        }
    }
}
