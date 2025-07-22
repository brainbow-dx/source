use std::collections::HashMap;

use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse::Result;
use syn::*;
// use syn::token::Brace;
// use syn::punctuated::Punctuated;

use quote::*;

//---
/// TODO
#[derive(Default)]
pub struct StylesBlock {
    /// TODO
    styles: HashMap<Ident, Vec<Expr>>,
}

impl StylesBlock {
    /// TODO
    pub fn new() -> Self {
        Self {
            styles: HashMap::new(),
        }
    }
}

impl Parse for StylesBlock {
    /// Lines starting with `#` are used to annotate statements within
    /// the block and may be used to alter how the block is built.
    fn parse(token_buf: ParseStream) -> Result<Self> {
        let mut block = StylesBlock::new();

        while !token_buf.is_empty() {
            if token_buf.peek(syn::Token![.]) {
                let mut class_names = Vec::new();

                loop {
                    if token_buf.peek(syn::Token![.]) {
                        token_buf.parse::<syn::Token![.]>()?;
                        class_names.push(token_buf.parse::<syn::Ident>()?);
                        continue;
                    }
                    if token_buf.peek(syn::Token![,]) {
                        token_buf.parse::<syn::Token![,]>()?;
                        continue;
                    }

                    // Nothing else allowed in the list.
                    // TODO: Better error here.
                    break;
                }

                let content;
                braced!(content in token_buf);
                let parsed_styles = content.parse_terminated(Expr::parse, syn::Token![,])?;

                for expr in parsed_styles {
                    for class_name in &class_names {
                        #[cfg(feature = "verbose")]
                        println!("Parsed Class Name: {:?}", class_name);

                        block
                            .styles
                            .entry(class_name.clone())
                            .or_insert_with(Vec::new)
                            .push(expr.clone());
                    }
                }

                #[cfg(feature = "verbose")]
                println!("Parsed styles: {:#?}", block.styles);

                // Start again in the same block,
                // after the parsed stylesheet ..
                continue;
            }
        }

        Ok(block)
    }
}

impl ToTokens for StylesBlock {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let styles = self.styles.iter().map(|(key, value)| {
            #[cfg(feature = "verbose")]
            println!("Printing Class Block Variable: {:?}", key);

            let values = value.iter().map(|value| {
                quote! {
                    stylesheet.push( #value );
                }
            });

            quote! {
                #[allow(unnecessary_braces)]
                let #key = move |stylesheet: &mut slate::style::StyleSheet<'_>| {
                    // Some Sensible imports for composability in the UIx block.
                    // TODO: Allow this to be set by the caller (with sensible defaults).
                    use slate::style::primitive::*;
                    use slate::style::primitive::Unit::*;
                    use slate::style::property::*;
                    use slate::event::EventKind::*;

                    #(#values)*
                };
            }
        });

        tokens.extend(quote! {
            // Some complex types require encapsulation in braces, parens, etc.
            // Allow a subset of these for convenience.
            #(#styles)*
        })
    }
}
