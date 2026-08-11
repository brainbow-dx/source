use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse::Result;
use syn::*;

use quote::*;

use crate::element::ElementClassAttribute;
use crate::element::ElementEventAttribute;
use crate::element::ElementProp;
use crate::element::ElementStyleAttribute;

//---
/// Global element index. Used to build unique identifiers for generated
/// names, doc comments, debug info, etc.
static ELEMENT_IDX: AtomicUsize = AtomicUsize::new(0);

//---
#[derive(Default)]
pub struct UIxBlock {
    roots: Vec<UIxElement>,
    styles: HashMap<Ident, Vec<Expr>>,
}

impl UIxBlock {
    const SELF_KEYWORD: &str = "self";

    pub fn new() -> Self {
        Self {
            roots: Vec::new(),
            styles: HashMap::new(),
        }
    }
}

impl Parse for UIxBlock {
    /// Lines starting with `#` are used to annotate statements within
    /// the block and may be used to alter how the block is built.
    fn parse(tokens: ParseStream) -> Result<Self> {
        let mut block = UIxBlock::new();

        while !tokens.is_empty() {
            if tokens.peek(syn::Token![.]) || tokens.peek(syn::Token![^]) {
                let mut class_names = Vec::new();

                loop {
                    if tokens.peek(syn::Token![^]) {
                        tokens.parse::<syn::Token![^]>()?;
                        tokens.parse::<syn::Token![self]>()?;
                        continue;
                    }
                    if tokens.peek(syn::Token![.]) {
                        tokens.parse::<syn::Token![.]>()?;
                        class_names.push(tokens.parse::<syn::Ident>()?);
                        continue;
                    }
                    if tokens.peek(syn::Token![,]) {
                        tokens.parse::<syn::Token![,]>()?;
                        continue;
                    }

                    break;
                }

                let content;
                braced!(content in tokens);
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

            match UIxElement::parse(tokens) {
                Ok(mut element) => {
                    element.root = true;
                    block.roots.push(element);
                }
                Err(error) => {
                    // Return an error if unable to recover
                    return Err(tokens.error(format!("Couldn't parse element: {:}", error)));
                }
            }
        }

        Ok(block)
    }
}

impl ToTokens for UIxBlock {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let roots = &self.roots;

        #[cfg(feature = "verbose")]
        for root_element in roots {
            println!("Root Element: {:#?}", root_element);
        }

        // Emits styling instructions for the current draw context.
        let styles = self.styles.iter().map(|(key, values)| {
            #[cfg(feature = "verbose")]
            println!("Printing Class Block Variable: {:?}", key);

            let values = values.iter().map(|value| {
                quote! {
                    stylesheet.push( #value );
                }
            });

            if key == UIxBlock::SELF_KEYWORD {
                // Emits a call to the parent scaffold to pass new styles to
                // the parent of the current element.
                quote! {
                    scaffold
                        .with_class_attr(move |stylesheet: &mut StyleSheet<'_>| {
                            #(#values)*
                        });
                }
            } else {
                // Emits a `class_fn`` to be passed to some child element of
                // the element tree in the current render scope.
                quote! {
                    let #key = move |stylesheet: &mut StyleSheet<'_>| {
                        #(#values)*
                    };
                }
            }
        });

        tokens.extend(quote! {
            #(#styles)*
            #(#roots)*
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UIxElement {
    pub ident: Ident,
    pub ctor: Option<ExprCall>,
    pub prefix: String,
    pub root: bool,
    pub index: usize,
    pub is_closed: bool,
    pub events: Vec<ElementEventAttribute>,
    pub classes: Vec<ElementClassAttribute>,
    pub styles: Vec<ElementStyleAttribute>,
    pub props: Vec<ElementProp>,
    pub children: Vec<UIxElement>,
}

impl UIxElement {
    pub fn new(ident: Ident, index: usize) -> Self {
        Self {
            ident,
            ctor: None,
            prefix: String::new(),
            root: false,
            index: index,
            is_closed: false,
            events: Vec::new(),
            styles: Vec::new(),
            classes: Vec::new(),
            props: Vec::new(),
            children: Vec::new(),
        }
    }
}

impl UIxElement {
    pub fn name(&self) -> String {
        format!("{:}_{:08}", self.prefix, self.index)
    }
}

impl Parse for UIxElement {
    /// Parses a single element, including any attributes, children, etc.
    ///
    /// ### Example (Input)
    /// ```rust
    /// #[on(Click, click_handler_fn)]
    /// #[style(BackgroundColor, hexa("#FF0000", 0.5))]
    /// #[class("my-class-name")]
    /// <ElementTestImpl name="Second Root" number=0usize>
    ///     etc ..
    /// </ElementTestImpl>
    fn parse(token_buf: ParseStream) -> Result<Self> {
        #[cfg(feature = "verbose")]
        println!("---\nParsing a new Element!");

        let mut events = Vec::new();
        let mut classes = Vec::new();
        let mut styles = Vec::new();

        // Parse attributes, like:
        // - Event handlers: `#[on:click(..)]`
        // - Inline Styles: `#[style(background-color: #FF0000)]`
        // - Class Names: `#[class("my-class-name")]`
        loop {
            if !token_buf.peek(syn::Token![#]) {
                break;
            }
            // Parse attributes, like `#[attr]` ..
            let attributes = Attribute::parse_outer(token_buf)?;

            // .. and print them for debugging.
            #[cfg(feature = "verbose")]
            if attributes.len() > 0 {
                println!("Parsed Attributes: {:#?}", attributes);
            }

            for attr in attributes {
                match attr.path().get_ident() {
                    Some(attr_ident) => {
                        match attr_ident.to_string().as_str() {
                            // Found UUID, as in `#[uuid(3500dad6-cdef-442a-ba05-d20c1f3b1921)]`.
                            // Explicitly sets a unique identifier on the element.
                            "tag" => println!("Parsed Tag Attribute: TODO"),
                            // Found When, as in `#[when(n == 1)]`.
                            // Allows an element to be rendered conditionally.
                            "when" => println!("Parsed While Attribute: TODO"),
                            // Found Each, as in `#[each(n in 0..4)]`.
                            // Allows an element to be rendered multiple times.
                            "each" => println!("Parsed For Attribute: TODO"),
                            // Found Event, as in `#[event(Kind, fn1, fn2, etc ..)]`.
                            // Register's an event listener on the element.
                            "on" => events.push(ElementEventAttribute::try_from(&attr)?),
                            // Found Class, as in `#[class(class_expr)]`.
                            // Adds a class name to the element.
                            "class" => classes.push(ElementClassAttribute::try_from(&attr)?),
                            // Found Style, as in `#[style(Prop, Value)]`.
                            // Sets a style property on the element.
                            "style" => styles.push(ElementStyleAttribute::try_from(&attr)?),
                            // Found Documentation, as in `#[doc = "TODO"]` or `/// Something`.
                            // Set a line of documentation on the element.
                            "doc" => {
                                #[cfg(feature = "verbose")]
                                println!("Parsed Documentation Attribute: TODO");
                            }
                            _ => {
                                return Err(token_buf.error(format!("Unknown attribute ident: {:}", attr_ident)));
                            }
                        }
                    }
                    None => {
                        return Err(token_buf.error(format!("Failed to parse attribute ident: {:?}", attr)));
                    }
                }
            }
        }

        // Parse the initial leading-tag `<` token.
        token_buf.parse::<syn::Token![<]>()?;

        // TODO: Parse a path here instead.
        let name = Ident::parse(token_buf)?;
        let index = ELEMENT_IDX.fetch_add(1, Ordering::SeqCst);

        let mut element = UIxElement::new(name, index);
        element.events.extend(events.drain(..));
        element.styles.extend(styles.drain(..));
        element.classes.extend(classes.drain(..));
        element.is_closed = false;

        if token_buf.peek(syn::Token![::]) {
            token_buf.parse::<syn::Token![::]>()?;
            element.ctor = Some(token_buf.parse::<syn::ExprCall>()?);
        }

        loop {
            // Is this the end of the opening tag of the element? `>`
            if token_buf.peek(syn::Token![>]) {
                token_buf.parse::<syn::Token![>]>()?;
                break;
            }

            // Is this the end of the element? `/>`?
            if token_buf.peek(syn::Token![/]) && token_buf.peek2(syn::Token![>]) {
                token_buf.parse::<syn::Token![/]>()?;
                token_buf.parse::<syn::Token![>]>()?;
                element.is_closed = true;
                break;
            }

            // Successfully parsed, so advance the main parser
            element.props.push(token_buf.parse::<ElementProp>()?);
        }

        #[cfg(feature = "verbose")]
        eprintln!("Parsed props: {:#?}", element.props);

        if !element.is_closed {
            // We don't yet have a close condition, meaning no `/>` was found.
            // Parse either the closing tag or a set of children.

            #[cfg(feature = "verbose")]
            println!("Not a self-closing tag. Attempting to parse children or closing tag.");

            loop {
                // Check for a pair of </ tokens and parse the close tag ..
                if token_buf.peek(syn::Token![<]) && token_buf.peek2(syn::Token![/]) {
                    #[cfg(feature = "verbose")]
                    println!("Parsing Closing Tag ..");

                    // Parse opening to the close tag as `</`.
                    token_buf.parse::<syn::Token![<]>()?;
                    token_buf.parse::<syn::Token![/]>()?;

                    let closing_ident = token_buf.parse::<syn::Ident>()?;
                    if element.ident != closing_ident {
                        return Err(token_buf.error(format!(
                            "Expected closing tag for ident `{:}`; found `{:}`",
                            element.ident, closing_ident
                        )));
                    }

                    token_buf.parse::<syn::Token![>]>()?;

                    #[cfg(feature = "verbose")]
                    println!("Closed Tag: {:#?}", closing_ident);

                    break; // Finished!
                } else {
                    #[cfg(feature = "verbose")]
                    println!("Parsing Child Element ..");

                    // Otherwise, attempt to match a child element.
                    match UIxElement::parse(token_buf) {
                        Ok(child_element) => {
                            #[cfg(feature = "verbose")]
                            println!("Parsed child Element: {:#?}", child_element);
                            element.children.push(child_element);
                        }
                        Err(error) => {
                            #[cfg(feature = "dev")]
                            eprintln!("Failed to parse child Element: {:#?}", error);
                            return Err(
                                token_buf.error(format!("Couldn't parse child of `{}`: {:}", element.ident, error))
                            );
                        }
                    }
                }
            }
        }

        #[cfg(feature = "dev")]
        #[cfg(feature = "verbose")]
        println!("Parsed Element: {:#?}", element);

        Ok(element)
    }
}

impl ToTokens for UIxElement {
    /// Generate a token stream for UIxElement composition.
    ///
    /// ### Example (Output)
    /// ```rust
    /// // Add the element to the scaffold ..
    /// scaffold.add({
    ///     SomeElement::default()
    ///         .with_name("Second Root")
    ///         .with_number(0usize)
    /// })
    ///     // .. along with event handlers, styles, etc ..
    ///     .with_event_attr(EventKind::Click, click_handler_fn)
    ///     .with_style_attr(StyleProperty::BackgroundColor, hexa("#ff0000", 0.5))
    ///     // .. and finally, add children.
    ///     .with_children(|some_element| {
    ///         // etc..
    ///     });
    /// ````
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ident = &self.ident;
        // let prefix = Ident::new(&self.prefix, ident.span());
        let var_name = Ident::new(&self.name(), ident.span());

        let ctor = self.ctor.to_owned().or_else(|| {
            Some(syn::parse_quote! {
                default()
            })
        });

        // Write props, where `prop="value"`, `prop={value}`, or `prop=[value]`
        // is converted to `.with_prop(value)`.
        let mut props = proc_macro2::TokenStream::new();
        for ElementProp {
            key,
            value,
        } in &self.props
        {
            // Build the method name into a "with" method (for building the element's props).
            let method_name = Ident::new(&format!("with_{}", key), key.span());
            match value {
                Some(value) => props.extend(quote! { . #method_name ( #value ) }),
                None => props.extend(quote! { . #method_name ( #key ) }),
            }
        }

        // Construct `.with_event(..)` calls for each event.
        let mut events = proc_macro2::TokenStream::new();
        for event in &self.events {
            let event_handler = event.handler();
            events.extend(quote! {
                .with_event_attr(#event_handler)?
            });
        }

        // Construct `.with_style(..)` calls for each style.
        let mut styles = proc_macro2::TokenStream::new();
        for style in &self.styles {
            let event_kind = style.expr();
            styles.extend(quote! {
                .with_style_attr(#event_kind)?
            });
        }

        // Construct `.with_style(..)` calls for each style.
        let mut classes = proc_macro2::TokenStream::new();
        for class_attrs in &self.classes {
            #[cfg(feature = "verbose")]
            println!("Class Expr: {:#?}", class_attrs);

            for class_expr in class_attrs.classes() {
                classes.extend(quote! {
                    .with_class_attr( #class_expr )?
                });
            }
        }

        // TODO: Write children, where a `<Child />` is converted to a scoped
        // block which builds additional nested elements.
        let children = &self.children;

        // TODO: Remove `.with_children()` when there are no children.
        tokens.extend(quote! {
            let #var_name = scaffold.add({
                #ident :: #ctor #props
            })?
                #events // .with_event_attr(..),*
                #styles // .with_style_attr(..),*
                #classes // .with_class_attr(..),*
                .with_children(|scaffold| {
                    #(#children)*
                    Ok(()) // TODO: Return a draw result.
                })?
                .build()?;
        });
    }
}
