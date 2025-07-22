use proc_macro2::TokenStream;

use quote::ToTokens;
use quote::quote;

use syn::Meta;
use syn::*;

use syn::Attribute;
use syn::Error;
use syn::Expr;
use syn::Result;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;

//---
/// TODO
pub struct ElementDeriveBuilder {
    /// TODO
    source: DeriveInput,
}

impl ElementDeriveBuilder {
    /// TODO
    pub fn new(source: DeriveInput) -> Self {
        ElementDeriveBuilder { source }
    }
}

impl ElementDeriveBuilder {
    /// TODO
    pub fn build(&self) -> TokenStream {
        let root_ident = self.source.ident.to_owned();

        match &self.source.data {
            // TODO
            Data::Struct(struct_element) => {
                for _field in struct_element.fields.iter() {
                    #[cfg(feature = "debug")]
                    println!("\x1b[1;35m     Element\x1b[0;0m {:?}", _field.ident);

                    //..
                }
            }

            // TODO: Add support for deriving from Enums, etc ..
            _ => {
                panic!("Element can only be derived for structs")
            }
        }

        let mut render_exprs = Vec::new();
        for attr in &self.source.attrs {
            match &attr.meta {
                Meta::List(list) => {
                    if list.path.is_ident("render") {
                        let expr_args = list
                            .parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)
                            .expect("attribute arguments");
                        if expr_args.len() < 1 || expr_args.len() > 2 {
                            panic!("Invalid arguments.");
                        }

                        let mut render_expr = None;
                        let mut render_condition = None;

                        for expr_arg in expr_args {
                            match expr_arg {
                                Expr::Path(_) | Expr::Field(_) if render_expr == None => {
                                    render_expr = Some(expr_arg);
                                }
                                Expr::If(arg_if)
                                    if render_expr != None && render_condition == None =>
                                {
                                    render_condition = Some(arg_if.cond);
                                }
                                Expr::Path(_) => {
                                    panic!("Path already exists!");
                                }
                                _ => {
                                    panic!("Unsupported argument: {:?}", expr_arg);
                                }
                            }
                        }

                        if let Some(render_expr) = render_expr {
                            render_exprs.push((render_expr, render_condition))
                        }
                    }
                }
                _ => {
                    //..
                }
            }
        }

        // TODO
        TokenStream::from(quote! {
            #[automatically_derived]
            impl core::fmt::Display for #root_ident {
                /// TODO
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.write_str("<#root_ident>")
                }
            }

            #[automatically_derived]
            impl Element for #root_ident {
                //..
            }
        })
    }

    // /// TODO
    // pub fn build_props(&self) -> TokenStream {
    //     TokenStream::from(quote! {
    //         //..
    //     })
    // }

    // /// TODO
    // pub fn build_children(&self) -> TokenStream {
    //     TokenStream::from(quote! {
    //         //..
    //     })
    // }
}

// enum ElementAttributeName {
//     Ident(Ident),
//     Keyword(Token![fn]),
// }

// impl Parse for ElementAttributeName {
//     fn parse(input: ParseStream) -> Result<Self> {
//         if input.peek(Ident) {
//             Ok(ElementAttributeName::Ident(input.parse()?))
//         } else if input.peek(Token![for]) {
//             Ok(ElementAttributeName::Keyword(input.parse()?))
//         } else if input.peek(Token![while]) {
//             Ok(ElementAttributeName::Keyword(input.parse()?))
//         } else {
//             Err(input.error("Expected an identifier or one of: `for`, `while`"))
//         }
//     }
// }

/// An event attribute, as in `#[event(Click, click_handler_fn)]`.
/// - Slot `0` is the event name, as in `Click`.
/// - Slot `1` is the event handler function, as in `fn click_handler_fn(..)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ElementEventAttribute(Expr);

impl ElementEventAttribute {
    /// TODO
    pub fn handler(&self) -> &Expr {
        &self.0
    }
}

impl TryFrom<&Attribute> for ElementEventAttribute {
    type Error = Error;

    /// TODO
    fn try_from(attr: &Attribute) -> Result<Self> {
        match &attr.meta {
            Meta::List(meta_list) => ElementEventAttribute::try_from(meta_list),
            _ => Err(Error::new_spanned(
                attr,
                "Expected a list of items for 'event'",
            )),
        }
    }
}

impl TryFrom<&MetaList> for ElementEventAttribute {
    type Error = Error;

    /// TODO
    fn try_from(meta_list: &MetaList) -> Result<Self> {
        // Events are seperated by commas, so we use `Punctuated` to parse them.
        let punctuated =
            meta_list.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?;

        // Extract the event name (required).
        let Some(first_expr) = punctuated.first().cloned() else {
            return Err(Error::new_spanned(meta_list, "Expected event name"));
        };

        Ok(ElementEventAttribute(first_expr))
    }
}

/// A style attribute, as in
/// ```
/// #[style(BackgroundColor, hexa("#FF0000", 1.0))]
/// ````
/// - Slot `0` is the style name, as in `BackgroundColor`.
/// - Slot `1` is one or more style values, as in `hexa("#FF0000", 1.0)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ElementStyleAttribute(Expr);

impl ElementStyleAttribute {
    /// TODO
    pub fn expr(&self) -> &Expr {
        &self.0
    }
}

impl TryFrom<&Attribute> for ElementStyleAttribute {
    type Error = Error;

    /// Get the style name and values from an `Attribute`.
    fn try_from(attr: &Attribute) -> Result<Self> {
        match &attr.meta {
            Meta::List(meta_list) => ElementStyleAttribute::try_from(meta_list),
            _ => Err(Error::new_spanned(
                attr,
                "Expected a list of items for 'event'",
            )),
        }
    }
}

impl TryFrom<&MetaList> for ElementStyleAttribute {
    type Error = Error;

    /// Get the style name and values from a `MetaList`.
    fn try_from(meta_list: &MetaList) -> Result<Self> {
        // Styles are seperated by commas, so we use `Punctuated` to parse them.
        let punctuated =
            meta_list.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?;

        // Extract the style name (required).
        let Some(first_expr) = punctuated.first().cloned() else {
            return Err(Error::new_spanned(meta_list, "Expected event name"));
        };

        // TODO: Extract addtional style values (optional).
        // Note: Probably do this in a collection type?
        // let other_exprs = punctuated.iter().skip(1).cloned().collect::<Vec<_>>();

        Ok(ElementStyleAttribute(first_expr))
    }
}

/// A class attribute, as in `#[class(style_buiilder_fn)]`.
/// - Slot `0` is the style builder function, as in `fn some_class_name(..)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ElementClassAttribute(Vec<Expr>);

impl ElementClassAttribute {
    /// TODO
    pub fn classes(&self) -> &Vec<Expr> {
        &self.0
    }
}

impl TryFrom<&Attribute> for ElementClassAttribute {
    type Error = Error;

    /// TODO
    fn try_from(attr: &Attribute) -> Result<Self> {
        match &attr.meta {
            Meta::List(meta_list) => ElementClassAttribute::try_from(meta_list),
            _ => Err(Error::new_spanned(
                attr,
                "Expected a list of expressions for 'class'",
            )),
        }
    }
}

impl TryFrom<&MetaList> for ElementClassAttribute {
    type Error = Error;

    /// TODO
    fn try_from(meta_list: &MetaList) -> Result<Self> {
        let punctuated =
            meta_list.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?;

        #[cfg(feature = "verbose")]
        for expr in &punctuated {
            println!("Parsed Expression: {:?}", expr);
        }

        let class_exprs = punctuated.iter().cloned().collect::<Vec<_>>();

        Ok(ElementClassAttribute(class_exprs))
    }
}

// impl ToTokens for ElementClassAttribute {
//     fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
//         let class_list = self.classes();
//         tokens.extend(quote! {
//             &[ #(#class_list),* ] as &[fn(&mut slate::style::StyleSheet)]
//         });
//     }
// }

//---
/// Represents a key/value pair (e.g., `key="value"`).
#[derive(Debug, Clone, PartialEq)]
pub struct ElementProp {
    /// TODO: Make private.
    pub key: Ident,

    /// TODO: Make private.
    pub value: Option<ElementPropValue>,
}

impl TryFrom<&ExprAssign> for ElementProp {
    type Error = Error;

    /// TODO
    fn try_from(expr_assign: &ExprAssign) -> Result<Self> {
        let key = match &*expr_assign.left {
            Expr::Path(expr_path) => match expr_path.path.get_ident() {
                Some(ident) => ident.to_owned(),
                None => {
                    return Err(Error::new_spanned(
                        expr_assign,
                        format!(
                            "Expected an identifier for the prop key but got `{:?}`.",
                            expr_path
                        ),
                    ));
                }
            },
            _ => {
                return Err(Error::new_spanned(
                    expr_assign,
                    "Expected an identifier for the prop key but got `None`.",
                ));
            }
        };

        let value = Some(ElementPropValue(*expr_assign.right.to_owned()));

        Ok(ElementProp { key, value })
    }
}

impl Parse for ElementProp {
    /// Parse a key/value pair formatted as `key[="value"]` to an `ElementProp`.
    fn parse(token_buf: ParseStream) -> Result<Self> {
        let key = Ident::parse(token_buf)?;
        let mut value = None;

        if token_buf.peek(syn::Token![=]) {
            token_buf.parse::<syn::Token![=]>()?;
            value = Some(ElementPropValue::parse(token_buf)?);
        }

        Ok(ElementProp { key, value })
    }
}

/// TODO
#[derive(Debug, Clone, PartialEq)]
pub struct ElementPropValue(Expr);

impl Parse for ElementPropValue {
    /// TODO
    fn parse(token_buf: ParseStream) -> Result<Self> {
        let prop_value_parsers: &[fn(ParseStream) -> Result<Expr>] = &[
            |buf| buf.parse::<ExprLit>().map(Expr::Lit),
            |buf| buf.parse::<ExprBlock>().map(Expr::Block),
            |buf| buf.parse::<ExprArray>().map(Expr::Array),
            |buf| buf.parse::<ExprParen>().map(Expr::Paren),
            |buf| buf.parse::<ExprPath>().map(Expr::Path),
            |buf| buf.parse::<ExprCall>().map(Expr::Call),
            |buf| buf.parse::<ExprMethodCall>().map(Expr::MethodCall),
            |buf| buf.parse::<ExprClosure>().map(Expr::Closure),
            |buf| buf.parse::<ExprIndex>().map(Expr::Index),
            |buf| buf.parse::<ExprField>().map(Expr::Field),
            |buf| buf.parse::<ExprReference>().map(Expr::Reference),
            |buf| buf.parse::<ExprUnary>().map(Expr::Unary),
            |buf| buf.parse::<ExprBinary>().map(Expr::Binary),
            |buf| buf.parse::<ExprTry>().map(Expr::Try),
            |buf| buf.parse::<ExprRange>().map(Expr::Range),
        ];

        let mut errors = Vec::with_capacity(prop_value_parsers.len());

        for parser in prop_value_parsers {
            match parser(token_buf) {
                Ok(expr) => return Ok(ElementPropValue(expr)),
                Err(error) => errors.push(error),
            };
        }

        Err(token_buf.error(format!(
            "Failed to parse prop value expression: {:?}",
            errors
        )))
    }
}

impl ToTokens for ElementPropValue {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ElementPropValue(expr) = self;
        tokens.extend(quote! { #expr });
    }
}
