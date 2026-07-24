use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Expr, Lit};

/// Custom derive macro to generate a method that copies env vars with a prefix.
///
/// Usage:
/// ```ignore
/// #[derive(clap::Args, EnvPrefix)]
/// #[env_prefix = "MY_APP"]
/// struct MyArgs { ... }
/// ```
///
/// This generates a `prepend_envs()` method that copies each env var
/// recognized by clap into a prefixed variant (e.g. `MY_APP_HOST`).
#[proc_macro_derive(EnvPrefix, attributes(env_prefix))]
pub fn derive_env_prefix(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let prefix = ast
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("env_prefix"))
        .and_then(|attr| {
            if let Ok(meta) = attr.meta.require_name_value() {
                if let Expr::Lit(expr_lit) = &meta.value {
                    if let Lit::Str(s) = &expr_lit.lit {
                        Some(s.value())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap_or_default();

    let name = &ast.ident;

    let r#gen = quote! {
        impl #name {
            pub fn prepend_envs() {
                use std::env;
                use clap::Args;

                let cmd = <Self as Args>::augment_args(clap::Command::new(""));
                for arg in cmd.get_arguments() {
                    if let Some(env_var) = arg.get_env() {
                        if let Some(env_name) = env_var.to_str() {
                            if let Ok(value) = env::var(env_name) {
                                let prefixed_var = format!("{}_{}", #prefix, env_name);
                                unsafe {
                                    env::set_var(&prefixed_var, &value);
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    r#gen.into()
}
