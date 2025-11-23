mod uix;

mod styles;

mod element;

//---
extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;

use syn::DeriveInput;
use syn::parse_macro_input;

use quote::quote;

//---
/// Derive Element for a struct.
#[proc_macro_derive(Element, attributes(element, prop, props, child, children, render))]
pub fn derive_element(token_buf: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let element = crate::element::ElementDeriveBuilder::new(syn::parse_macro_input!(token_buf));

    #[cfg(feature = "debug")]
    println!("Attempting to derive Element from: {:?}", element.ast.ident);

    element.build().into()
}

/// Write a UIx block using a JSX-like syntax.
#[proc_macro]
pub fn uix(token_buf: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match syn::parse::<crate::uix::UIxBlock>(token_buf) {
        Ok(uix_block) => {
            // TODO
            proc_macro::TokenStream::from(quote! {
                // Outer block helps keep the parent use-space pure.
                {
                    // Some complex types require encapsulation in braces, parens, etc.
                    // Allow a subset of these for convenience.
                    #[allow(unused_braces)]
                    #[allow(unused_parens)]
                    move |scaffold| {
                        #uix_block
                        Ok(())
                    }
                }
            })
        }
        Err(error) => {
            // TODO: Print the token_buf token stream for debugging
            // println!("Failed to parse the following token_buf: {:?}", input_clone);

            // Convert the syn::Error into a compiler error
            error.to_compile_error().into() // </3
        }
    }
}

/// Write a UIx block using a JSX-like syntax.
#[proc_macro]
pub fn styles(token_buf: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match syn::parse::<crate::styles::StylesBlock>(token_buf) {
        Ok(styles_block) => {
            // TODO
            proc_macro::TokenStream::from(quote! {
                #styles_block
            })
        }
        Err(error) => {
            // Print the token_buf token stream for debugging
            // println!("Failed to parse the following token_buf: {:?}", input_clone);
            // Convert the syn::Error into a compiler error
            let compile_error = error.to_compile_error();
            compile_error.into() // </3
        }
    }
}

/// TODO
#[proc_macro_attribute]
pub fn render(_ts1: proc_macro::TokenStream, _ts2: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // _ts1
    proc_macro::TokenStream::new()
}

#[proc_macro_derive(StyleProperty)]
pub fn derive_style_property(input: TokenStream) -> TokenStream {
    let _ast = parse_macro_input!(input as DeriveInput);
    // TODO: Generate code for new typemap feature
    let r#gen = quote! {
        // Generated code goes here
    };
    r#gen.into()
}

#[proc_macro_derive(Unit)]
pub fn derive_unit(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let r#gen = quote! {
        #[automatically_derived]
        impl #name {
            pub fn new<U: Into<Unit>>(value: U) -> Self {
                Self(value.into())
            }

            pub fn unit(&self) -> &Unit {
                &self.0
            }
        }

        #[automatically_derived]
        impl From<Unit> for #name {
            fn from(unit: Unit) -> Self {
                Self(unit)
            }
        }

        #[automatically_derived]
        impl Into<Unit> for #name {
            fn into(self) -> Unit {
                self.0
            }
        }

        #[automatically_derived]
        impl Deref for #name {
            type Target = Unit<f32>;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        #[automatically_derived]
        impl Into<StyleValue> for #name {
            fn into(self) -> StyleValue {
                StyleValue::#name(self)
            }
        }

        #[automatically_derived]
        impl Style for #name {
            //..
        }
    };

    r#gen.into()
}

#[proc_macro_derive(Rect)]
pub fn derive_rect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let r#gen = quote! {
        #[automatically_derived]
        impl #name {
            /// TODO
            #[inline(always)]
            pub fn new<U: Into<Unit<f32>>>(all: U) -> Self {
                let all = all.into();
                #name ::all(all, all, all, all)
            }

            /// TODO
            #[inline(always)]
            pub fn all<U1: Into<Unit<f32>>, U2: Into<Unit<f32>>, U3: Into<Unit<f32>>, U4: Into<Unit<f32>>>(top: U1, right: U2, bottom: U3, left: U4) -> Self {
                #name(Rect::all(top, right, bottom, left))
            }

            /// TODO
            #[inline(always)]
            pub fn xy<U1: Into<Unit<f32>> + Copy, U2: Into<Unit<f32>> + Copy>(x: U1, y: U2) -> Self {
                #name(Rect::all(y, x, y, x))
            }
        }

        #[automatically_derived]
        impl #name {
            /// TODO
            #[inline(always)]
            pub fn rect<'rect>(&'rect self) -> &'rect Rect {
                &self.0
            }
        }

        #[automatically_derived]
        impl Deref for #name {
            type Target = Rect;

            /// TODO
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        #[automatically_derived]
        impl Into<Rect> for #name {
            /// TODO
            fn into(self) -> Rect {
                self.0
            }
        }

        #[automatically_derived]
        impl Into<StyleValue> for #name {
            /// TODO
            fn into(self) -> StyleValue {
                StyleValue::#name(self)
            }
        }

        #[automatically_derived]
        impl Style for #name {
            //..
        }
    };

    r#gen.into()
}

#[proc_macro_derive(Size2d)]
pub fn derive_size2d(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let r#gen = quote! {
        impl #name {
            /// TODO
            #[inline(always)]
            pub fn new<U1: Into<Unit<f32>>, U2: Into<Unit<f32>>>(x: U1, y: U2) -> Self {
                #name ::xy(x, y)
            }

            /// TODO
            #[inline(always)]
            pub fn both<U1: Into<Unit<f32>>, U2: Into<Unit<f32>>>(x: U1, y: U2) -> Self {
                #name ::xy(x, y)
            }

            /// TODO
            #[inline(always)]
            pub fn xy<U1: Into<Unit<f32>>, U2: Into<Unit<f32>>>(x: U1, y: U2) -> Self {
                #name(Size2d(x.into(), y.into()))
            }
        }

        impl Deref for #name {
            type Target = Size2d;

            /// TODO
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl #name {
            /// TODO
            #[inline(always)]
            pub fn get_size_2d<'rect>(&'rect self) -> &'rect Size2d {
                &self.0
            }
        }

        impl Into<StyleValue> for #name {
            /// TODO
            fn into(self) -> StyleValue {
                StyleValue::#name(self)
            }
        }

        impl Style for #name {
            //..
        }
    };

    r#gen.into()
}

#[proc_macro_derive(Color)]
pub fn derive_color(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    // Extract the name of the struct
    let name = &ast.ident;

    // Generate code
    let r#gen = quote! {
        #[automatically_derived]
        impl #name {
            /// TODO
            #[inline(always)]
            pub fn hex(hex: &str) -> Self {
                #name(Color::hex(hex).unwrap_or(Color::Transparent))
            }
        }

        #[automatically_derived]
        impl #name {
            /// TODO
            pub fn color(&self) -> &Color {
                &self.0
            }
        }

        #[automatically_derived]
        impl Deref for #name {
            type Target = Color;

            /// TODO
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        #[automatically_derived]
        impl Into<StyleValue> for #name {
            /// TODO
            fn into(self) -> StyleValue {
                StyleValue::#name(self)
            }
        }

        #[automatically_derived]
        impl Style for #name {
            //..
        }
    };

    r#gen.into()
}
