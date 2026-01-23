extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type, TypePath};

#[proc_macro_derive(Memory)]
pub fn memory_derive(input: TokenStream) -> TokenStream {
	let ast = parse_macro_input!(input as DeriveInput);
	let name = &ast.ident;

	let expanded = quote! {
		use egui::{Context, Id};
		impl Memory for #name {
			fn load_or_default(ctx: &Context, id: Id) -> Self {
				ctx.data_mut(|d| d.get_persisted_mut_or(id, Self::default()).clone())
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

#[proc_macro_derive(TempMemory)]
pub fn temp_memory_derive(input: TokenStream) -> TokenStream {
	let ast = parse_macro_input!(input as DeriveInput);
	let name = &ast.ident;

	let expanded = quote! {
		use egui::{Context, Id};
		impl TempMemory for #name {
			fn load_or_default(ctx: &Context, id: Id) -> Self {
				ctx.data_mut(|d| d.get_temp_mut_or(id, Self::default()).clone())
			}

			fn load(ctx: &Context, id: Id) -> Option<Self> {
				ctx.data_mut(|d| d.get_temp(id))
			}

			fn store(self, ctx: &Context, id: Id) {
				ctx.data_mut(|d| d.insert_temp(id, self));
			}
		}
	};

	expanded.into()
}

#[proc_macro_derive(Id)]
pub fn id_derive(input: TokenStream) -> TokenStream {
	let ast = parse_macro_input!(input as DeriveInput);

	let Data::Struct(data_struct) = &ast.data else {
		return syn::Error::new(ast.ident.span(), "Expected tuple struct.")
			.to_compile_error()
			.into();
	};

	let Fields::Unnamed(fields) = &data_struct.fields else {
		return syn::Error::new(ast.ident.span(), "Expected tuple struct.")
			.to_compile_error()
			.into();
	};

	if fields.unnamed.len() != 1 {
		return syn::Error::new(ast.ident.span(), "Tuple struct must have exactly one field.")
			.to_compile_error()
			.into();
	}

	let ty = &fields.unnamed.first().unwrap().ty;

	if !is_integer_type(ty) {
		return syn::Error::new(ast.ident.span(), "Tuple field must be integer type.")
			.to_compile_error()
			.into();
	}

	let ty_name = ty.to_token_stream();
	let name = &ast.ident;

	quote! {
		impl std::fmt::Display for #name {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				write!(f, "{}", self.0)
			}
		}

		impl From<#ty_name> for #name {
			fn from(value: #ty_name) -> Self {
				Self(value)
			}
		}

		impl std::str::FromStr for #name {
			type Err = std::num::ParseIntError;

			fn from_str(s: &str) -> Result<Self, Self::Err> {
				#ty_name::from_str(s).map(|value| value.into())
			}
		}
	}
	.into()
}

fn is_integer_type(ty: &Type) -> bool {
	if let Type::Path(TypePath { path, .. }) = ty {
		if let Some(segment) = path.segments.first() {
			return matches!(
				segment.ident.to_string().as_str(),
				"i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128"
			);
		}
	}
	false
}
