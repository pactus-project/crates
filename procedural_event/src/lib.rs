use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{DeriveInput, Lit};

#[proc_macro_derive(Event, attributes(event_key))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let ident = &ast.ident;
    match event_key(ast.clone()) {
        Ok(event_key) => {
            let gen_code = quote! {
                impl #ident {
                    pub const event_key: &'static str = #event_key;
                }
                impl EventMessage for #ident {
                    fn key(&self) -> String { #event_key.to_string() }
                    fn as_any(&self) -> &dyn std::any::Any { self }
                }
            };
            gen_code.into()
        }
        Err(msg) => {
            let tokens = quote_spanned! { proc_macro2::Span::call_site() =>
                compile_error!(#msg);
            };
            tokens.into()
        }
    }
}

/// handle both attr formats:
/// - #[event_key("foo:bar")]
/// - #[event_key = "foo:bar"]
pub(crate) fn event_key(input: DeriveInput) -> Result<Lit, String> {
    let attrs = input.attrs;

    // Find the event_key attribute
    match attrs.iter().find(|attr| attr.path().is_ident("event_key")) {
        Some(attr) => {
            // Try to parse as a string literal directly
            match attr.parse_args::<Lit>() {
                Ok(lit) => Ok(lit),
                Err(_) => {
                    // If direct parsing fails, try the more complex approach
                    let meta = attr.meta.clone();
                    match meta.require_list() {
                        Ok(list) => match list.parse_args::<Lit>() {
                            Ok(lit) => Ok(lit),
                            Err(_) => Err(
                                "Expected a literal string for event_key, e.g., #[event_key(\"my:event\")]"
                                    .to_owned(),
                            ),
                        },
                        Err(_) => Err(
                            "Expected a literal string for event_key, e.g., #[event_key(\"my:event\")]"
                                .to_owned(),
                        ),
                    }
                }
            }
        }
        None => Err("Need event_key attribute".to_owned()),
    }
}
