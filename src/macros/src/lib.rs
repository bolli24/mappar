extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Memory)]
pub fn memory_derive(input: TokenStream) -> TokenStream {
	let ast = parse_macro_input!(input as DeriveInput);
	let name = &ast.ident;

	let expanded = quote! {
		use egui::{Context, Id};
		impl<'a> Memory<'a> for #name {
			fn load_or_default(ctx: &Context, id: Id) -> Self {
				ctx.data_mut(|d| *d.get_persisted_mut_or(id, Self::default()))
			}

			fn load(ctx: &Context, id: Id) -> Option<Self> {
				ctx.data_mut(|d| d.get_persisted(id))
			}

			fn store(self, ctx: &Context, id: Id) {
				ctx.data_mut(|d| d.insert_persisted(id, self));
			}
		}
	};

	expanded.into()
}
