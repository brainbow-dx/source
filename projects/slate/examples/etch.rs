#![feature(fn_traits)]
#![feature(unboxed_closures)]

extern crate alloc;

use std::process::ExitCode;

use slate::draw::Bump;
use slate::scaffold::Scaffold;
use slate::scaffold::ScaffoldError;

#[derive(oops::Error, derive_more::From)]
enum DrawError {
    #[msg("unknown error: {0}")]
    ScaffoldError(ScaffoldError),

    #[msg("unknown error: {0}")]
    Unknown(eyre::Error),
}

#[derive(Default)]
struct TerminalSurface;
impl TerminalSurface {
    fn draw<F>(&mut self, draw_fn: F) -> Result<(), DrawError>
    where
        F: Fn(Scaffold) -> Scaffold,
    {
        tracing::warn!("TODO: Implement `TerminalSurface`");

        let bump = Bump::new();
        let root = Scaffold::new_in(&bump);
        let root = draw_fn(root);
        
        // TODO: This should be done internally ..
        let _root = root.build();
        #[cfg(feature = "verbose")]
        {
            tracing::info!("Drawing Scaffold:");
            tracing::debug!("{:#?}", _root);
        }
        
        Ok(())
    }
}

//---
fn main() -> eyre::Result<ExitCode> {
    slate::log::init("TRACE");

    let mut terminal = TerminalSurface::default();

    terminal.draw(etch_splash_screen())?;

    Ok(ExitCode::SUCCESS)
}

fn etch_splash_screen() -> impl Fn(Scaffold) -> Scaffold {
    // slate::uix! {
    //     <Container>
    //         <Text />
    //     </Container>
    // }
    |s| s
}

#[allow(unused)]
#[automatically_derived]
fn etch_splash_screen_derived() -> impl Fn(Scaffold) -> Scaffold {
    move |scaffold| {
        // scaffold
        //     .add(Container::default())?
        //     .with_children(|scaffold| {
        //         scaffold
        //             .add(Text::default())?
        //             .build()?;
        //         Ok(())
        //     })?
        //     .build()?;
        scaffold
    }
}
