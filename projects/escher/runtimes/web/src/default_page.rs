//! The placeholder scaffold a newly-created page starts from. Used by both `surface::mount_scaffold`
//! (wasm) and `ssg::render_default_fragment` (native).

use escher_core::draw::Bump;
use escher_core::scaffold::Scaffold;
use escher_core::style::BackgroundColor;
use escher_core::style::ContentColor;
use escher_core::style::FlexDirection;
use escher_core::style::Gap;
use escher_core::style::Padding;
use escher_core::style::Value;

pub struct HeadingSlot;
pub struct BodySlot;

pub fn build_page_scaffold(arena: &Bump) -> Scaffold<'_> {
    Scaffold::new_in(arena)
        .style(Padding::all(24))
        .style(Gap(Value::from(12)))
        .style(FlexDirection::Column)
        .style(BackgroundColor::try_from("#1a1a1a").unwrap_or_default())
        .slot::<HeadingSlot>(|slot| {
            slot.content(Some("New Escher Page"))
                .style(ContentColor::try_from("#f5f5f5").unwrap_or_default())
        })
        .slot::<BodySlot>(|slot| {
            slot.content(Some("This page is rendered by a Scaffold, mounted through the escher-web wasm build."))
                .style(ContentColor::try_from("#a0a0a0").unwrap_or_default())
        })
}
