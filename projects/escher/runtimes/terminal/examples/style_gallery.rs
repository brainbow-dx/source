use std::process::ExitCode;
use std::io::Stdout;

use color_eyre::Result;

use ratatui::backend::CrosstermBackend;

use escher_core::surface::Surface;
use escher_core::style::*;

use escher_terminal::app::TerminalAction;
use escher_terminal::app::TerminalApp;
use escher_terminal::surface::TerminalSurface;

// A static gallery exercising every style property finished/added in this pass — Padding,
// per-edge Margin, Flex grow, FontWeight, TextDecorationLine, TextAlign, Heading, and
// Overflow::Scroll + ScrollPosition. Not interactive beyond the usual Escape-to-quit (handled
// by the terminal runtime itself, not this example). If a section looks wrong, that's the bug
// to chase — this file is meant to be read alongside the render output, not just run once.

fn main() -> Result<ExitCode> {
    color_eyre::install()?;

    TerminalApp::new()
        .with_surface(TerminalSurface::<CrosstermBackend<Stdout>>::try_default()?)
        .run(|surface| draw_gallery(surface))?;

    Ok(ExitCode::SUCCESS)
}

fn draw_gallery(surface: &mut TerminalSurface<CrosstermBackend<Stdout>>) -> Result<TerminalAction> {
    surface.draw(|root| {
        root
            .with_slot::<Title>(|title| {
                title
                    .with_style(Size::height(1))
                    .with_style(TextAlign::Center)
                    .with_style(FontWeight::Bold)
                    .with_content(Some("Escher Style Gallery"))
            })
            .with_slot::<PaddingDemo>(|demo| {
                demo
                    .with_style(Size::height(8))
                    .with_style(Border::new(1, BorderStyle::Solid, None))
                    .with_style(Padding::all(2))
                    .with_content(Some(
                        "Padding::all(2) — this text should sit 2 cells in from every border edge."
                    ))
            })
            .with_slot::<PaddingBgDemo>(|demo| {
                demo
                    .with_style(Size::height(6))
                    .with_style(Border::new(1, BorderStyle::Solid, None))
                    .with_style(Padding::all(1))
                    .with_style(BackgroundColor::from("#223355"))
                    .with_content(Some(
                        "Padding::all(1) + Border + BackgroundColor together — regression coverage for a real fixed bug (see spec/.agents/changelog.md). Every row inside the border should show the same blue background."
                    ))
            })
            .with_slot::<MarginDemo>(|demo| {
                demo
                    .with_style(FlexDirection::Row)
                    .with_style(Size::height(5))
                    .with_slot::<MarginBox>(|b| {
                        b.with_style(Border::new(1, BorderStyle::Solid, None))
                            .with_style(Margin::top(2))
                            .with_content(Some("Margin::top(2)"))
                    })
                    .with_slot::<MarginBox>(|b| {
                        b.with_style(Border::new(1, BorderStyle::Solid, None))
                            .with_style(Margin::left(4))
                            .with_content(Some("Margin::left(4)"))
                    })
                    .with_slot::<MarginBox>(|b| {
                        b.with_style(Border::new(1, BorderStyle::Solid, None))
                            .with_style(Margin::all(1))
                            .with_content(Some("Margin::all(1)"))
                    })
            })
            .with_slot::<FlexDemo>(|demo| {
                demo
                    .with_style(FlexDirection::Row)
                    .with_style(Size::height(3))
                    .with_slot::<FlexBox>(|b| {
                        b.with_style(Border::new(1, BorderStyle::Solid, None))
                            .with_style(Flex::new(1))
                            .with_content(Some("flex: 1"))
                    })
                    .with_slot::<FlexBox>(|b| {
                        b.with_style(Border::new(1, BorderStyle::Solid, None))
                            .with_style(Flex::new(2))
                            .with_content(Some("flex: 2 (should be ~2x as wide)"))
                    })
                    .with_slot::<FlexBox>(|b| {
                        b.with_style(Border::new(1, BorderStyle::Solid, None))
                            .with_style(Flex::new(1))
                            .with_content(Some("flex: 1"))
                    })
            })
            .with_slot::<TypographyDemo>(|demo| {
                demo
                    .with_style(Size::height(4))
                    .with_slot::<TypeLine>(|l| l.with_style(FontWeight::Bold).with_content(Some("Bold (FontWeight::Bold)")))
                    .with_slot::<TypeLine>(|l| l.with_style(TextDecorationLine::Underline).with_content(Some("Underline (TextDecorationLine::Underline)")))
                    .with_slot::<TypeLine>(|l| l.with_style(TextDecorationLine::LineThrough).with_content(Some("Strikethrough (TextDecorationLine::LineThrough)")))
                    .with_slot::<TypeLine>(|l| {
                        l.with_style(FontWeight::Bold)
                            .with_style(TextDecorationLine::Underline)
                            .with_content(Some("Bold + Underline together"))
                    })
            })
            .with_slot::<AlignDemo>(|demo| {
                demo
                    .with_style(Size::height(3))
                    .with_slot::<AlignLine>(|l| l.with_style(TextAlign::Left).with_content(Some("TextAlign::Left")))
                    .with_slot::<AlignLine>(|l| l.with_style(TextAlign::Center).with_content(Some("TextAlign::Center")))
                    .with_slot::<AlignLine>(|l| l.with_style(TextAlign::Right).with_content(Some("TextAlign::Right")))
            })
            .with_slot::<HeadingDemo>(|demo| {
                demo
                    .with_style(Size::height(2))
                    .with_style(Heading::H1)
                    .with_content(Some("This Is A Heading (Heading::H1 — renders bold; terminals have no font-size scale)"))
            })
            .with_slot::<ScrollDemo>(|demo| {
                demo
                    .with_style(Size::height(4))
                    .with_style(Border::new(1, BorderStyle::Solid, None))
                    .with_style(Overflow::Scroll)
                    .with_style(ScrollPosition::new(2))
                    .with_content(Some(
                        "Line 1 (scrolled past)\nLine 2 (scrolled past)\nLine 3 (should be the first visible line)\nLine 4\nLine 5\nLine 6"
                    ))
            })
            // Overlay regression coverage: unlike the slots above, this sits *on top of*
            // already-drawn content (the whole gallery behind it) — matches the shape the real
            // bug (assistant.rs's tasks/autocomplete overlay) actually showed up in.
            .with_overlay(|overlay| {
                overlay
                    .with_style(Size(30.into(), 6.into(), Value::Auto))
                    .with_style(Border::new(1, BorderStyle::Solid, None))
                    .with_style(Padding::all(1))
                    .with_style(BackgroundColor::from("#223355"))
                    .with_content(Some(
                        "Overlay + Padding::all(1) + Border + BackgroundColor — every row should be the same blue."
                    ))
            })
    })
}

//---
struct Title;
struct PaddingDemo;
struct PaddingBgDemo;
struct MarginDemo;
struct MarginBox;
struct FlexDemo;
struct FlexBox;
struct TypographyDemo;
struct TypeLine;
struct AlignDemo;
struct AlignLine;
struct HeadingDemo;
struct ScrollDemo;
