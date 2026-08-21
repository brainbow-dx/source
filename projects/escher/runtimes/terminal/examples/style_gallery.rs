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
            .slot::<Title>(|title| {
                title
                    .style(Size::height(1))
                    .style(TextAlign::Center)
                    .style(FontWeight::Bold)
                    .content(Some("Escher Style Gallery"))
            })
            .slot::<PaddingDemo>(|demo| {
                demo
                    .style(Size::height(8))
                    .style(Border::new(1, BorderStyle::Solid, None))
                    .style(Padding::all(2))
                    .content(Some(
                        "Padding::all(2) — this text should sit 2 cells in from every border edge."
                    ))
            })
            .slot::<PaddingBgDemo>(|demo| {
                demo
                    .style(Size::height(6))
                    .style(Border::new(1, BorderStyle::Solid, None))
                    .style(Padding::all(1))
                    .style(BackgroundColor::from("#223355"))
                    .content(Some(
                        "Padding::all(1) + Border + BackgroundColor together — regression coverage for a real fixed bug (see spec/.agents/changelog.md). Every row inside the border should show the same blue background."
                    ))
            })
            .slot::<MarginDemo>(|demo| {
                demo
                    .style(FlexDirection::Row)
                    .style(Size::height(5))
                    .slot::<MarginBox>(|b| {
                        b.style(Border::new(1, BorderStyle::Solid, None))
                            .style(Margin::top(2))
                            .content(Some("Margin::top(2)"))
                    })
                    .slot::<MarginBox>(|b| {
                        b.style(Border::new(1, BorderStyle::Solid, None))
                            .style(Margin::left(4))
                            .content(Some("Margin::left(4)"))
                    })
                    .slot::<MarginBox>(|b| {
                        b.style(Border::new(1, BorderStyle::Solid, None))
                            .style(Margin::all(1))
                            .content(Some("Margin::all(1)"))
                    })
            })
            .slot::<FlexDemo>(|demo| {
                demo
                    .style(FlexDirection::Row)
                    .style(Size::height(3))
                    .slot::<FlexBox>(|b| {
                        b.style(Border::new(1, BorderStyle::Solid, None))
                            .style(Flex::new(1))
                            .content(Some("flex: 1"))
                    })
                    .slot::<FlexBox>(|b| {
                        b.style(Border::new(1, BorderStyle::Solid, None))
                            .style(Flex::new(2))
                            .content(Some("flex: 2 (should be ~2x as wide)"))
                    })
                    .slot::<FlexBox>(|b| {
                        b.style(Border::new(1, BorderStyle::Solid, None))
                            .style(Flex::new(1))
                            .content(Some("flex: 1"))
                    })
            })
            .slot::<TypographyDemo>(|demo| {
                demo
                    .style(Size::height(4))
                    .slot::<TypeLine>(|l| l.style(FontWeight::Bold).content(Some("Bold (FontWeight::Bold)")))
                    .slot::<TypeLine>(|l| l.style(TextDecorationLine::Underline).content(Some("Underline (TextDecorationLine::Underline)")))
                    .slot::<TypeLine>(|l| l.style(TextDecorationLine::LineThrough).content(Some("Strikethrough (TextDecorationLine::LineThrough)")))
                    .slot::<TypeLine>(|l| {
                        l.style(FontWeight::Bold)
                            .style(TextDecorationLine::Underline)
                            .content(Some("Bold + Underline together"))
                    })
            })
            .slot::<AlignDemo>(|demo| {
                demo
                    .style(Size::height(3))
                    .slot::<AlignLine>(|l| l.style(TextAlign::Left).content(Some("TextAlign::Left")))
                    .slot::<AlignLine>(|l| l.style(TextAlign::Center).content(Some("TextAlign::Center")))
                    .slot::<AlignLine>(|l| l.style(TextAlign::Right).content(Some("TextAlign::Right")))
            })
            .slot::<HeadingDemo>(|demo| {
                demo
                    .style(Size::height(2))
                    .style(Heading::H1)
                    .content(Some("This Is A Heading (Heading::H1 — renders bold; terminals have no font-size scale)"))
            })
            .slot::<ScrollDemo>(|demo| {
                demo
                    .style(Size::height(4))
                    .style(Border::new(1, BorderStyle::Solid, None))
                    .style(Overflow::Scroll)
                    .style(ScrollPosition::new(2))
                    .content(Some(
                        "Line 1 (scrolled past)\nLine 2 (scrolled past)\nLine 3 (should be the first visible line)\nLine 4\nLine 5\nLine 6"
                    ))
            })
            // Overlay regression coverage: unlike the slots above, this sits *on top of*
            // already-drawn content (the whole gallery behind it) — matches the shape the real
            // bug (assistant.rs's tasks/autocomplete overlay) actually showed up in.
            .overlay(|overlay| {
                overlay
                    .style(Size(30.into(), 6.into(), Value::Auto))
                    .style(Border::new(1, BorderStyle::Solid, None))
                    .style(Padding::all(1))
                    .style(BackgroundColor::from("#223355"))
                    .content(Some(
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
