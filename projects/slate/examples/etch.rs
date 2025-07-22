#![feature(fn_traits)]
#![feature(unboxed_closures)]

extern crate alloc;

use std::process::ExitCode;

use slate::context::Bump;
use slate::element::Container;
use slate::element::DrawFn;
use slate::element::TextBlock;
use slate::scaffold::Scaffold;
use slate::scaffold::ScaffoldError;

#[derive(oops::Error, derive_more::From)]
enum DrawError {
    #[msg("unknown error: {0}")]
    ScaffoldError(ScaffoldError),

    #[msg("unknown error: {0}")]
    Unknown(anyhow::Error),
}

#[derive(Default)]
struct TerminalSurface;
impl TerminalSurface {
    fn draw<F>(&mut self, draw_fn: F) -> Result<(), DrawError>
    where
        F: Fn(&mut Scaffold) -> Result<(), ScaffoldError>,
    {
        tracing::warn!("TODO: Implement `TerminalSurface`");

        let bump = Bump::new();
        let mut root = Scaffold::new_in(&bump);
        draw_fn(&mut root).map_err(DrawError::ScaffoldError)?;

        let _root = root.build()?;
        #[cfg(feature = "verbose")]
        {
            tracing::info!("Drawing Scaffold:");
            tracing::debug!("{:#?}", _root);
        }

        Ok(())
    }
}

//---
fn main() -> anyhow::Result<ExitCode> {
    slate::log::init("TRACE");

    let mut terminal = TerminalSurface::default();

    terminal.draw(etch_splash_screen())?;

    Ok(ExitCode::SUCCESS)
}

fn etch_splash_screen() -> DrawFn {
    slate::uix! {
        <Container>
            <TextBlock />
        </Container>
    }
}

#[allow(unused)]
#[automatically_derived]
fn etch_splash_screen_derived() -> DrawFn {
    move |scaffold: &mut Scaffold| {
        scaffold
            .add(Container::default())?
            .with_children(|scaffold| {
                scaffold
                    .add(TextBlock::default())?
                    .build()?;
                Ok(())
            })?
            .build()?;
        Ok(())
    }
}
