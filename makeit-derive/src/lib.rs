use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{spanned::Spanned, GenericParam, ItemStruct};

fn capitalize(ident: &Ident) -> String {
    let ident = ident.to_string();
    let trimmed = ident.trim_start_matches("r#");

    let mut chars = trimmed.chars();
    match chars.next() {
        // FIXME: Should not be possible for an identifier. Return an error or use unwrap_unchecked?
        None => String::new(),
        Some(ch) => ch.to_uppercase().chain(chars).collect(),
    }
}

#[proc_macro_derive(Builder, attributes(default))]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    let input: ItemStruct = syn::parse(input).unwrap();
    let struct_name = &input.ident;
    let builder_name = format_ident!("{}Builder", input.ident);
    // We'll nest the entirety of the builder's helper types in a private module so that they don't
    // leak into the user's scope.
    let mod_name = format_ident!("{}Fields", builder_name);

    let mut struct_generics = input.generics.params.to_token_stream();
    if !input.generics.params.is_empty() {
        // Add a trailing comma to be able to add generics after it.
        struct_generics.extend(Some(quote!(,)));
    }

    // TODO: Use with_capacity where possible.
    // The type parameter names representing each field of the type being built.
    let mut set_fields_generics = vec![];
    // The type names representing fields that have been initialized.
    let mut all_set = vec![];
    // The type names representing fields that have not yet been initialized.
    let mut all_unset = vec![];

    // These are the generic parameters for the `impl` that lets the user call `.build()`. They
    // normally would all have to be "field_foo_set" and need no params beyond the underlying
    // type's, but we support default values so we need to account for them to let people build
    // without setting those.
    let mut buildable_generics = vec![];
    let mut buildable_generics_use = vec![];

    let mut default_where_clauses = vec![];

    for (i, field) in input.fields.iter().enumerate() {
        // We'll use these as the name of the type parameters for the builder's fields.
        let field_name = format_ident!(
            "{}",
            match &field.ident {
                Some(field) => capitalize(field),
                None => format!("Field{}", i), // Idents can't start with numbers.
            }
        );
        // We'll use these as the base for the types representing the builder state.
        let field_generic_name = format_ident!(
            "Field{}",
            match &field.ident {
                Some(field) => capitalize(field),
                None => format!("{}", i),
            }
        );
        let set_field_generic_name = format_ident!("{}Set", field_name);
        let unset_field_generic_name = format_ident!("{}Unset", field_name);

        let default_attr = field.attrs.iter().find_map(|attr| match &attr.meta {
            syn::Meta::Path(path) if path.is_ident("default") => Some(true),
            syn::Meta::List(syn::MetaList { path, .. }) if path.is_ident("default") => Some(false),
            _ => None,
        });
        if let Some(default_is_empty) = default_attr {
            let ty = &field.ty;
            buildable_generics.push(field_generic_name.clone());
            buildable_generics_use.push(field_generic_name.clone());
            if default_is_empty {
                default_where_clauses
                    .push(quote_spanned!(ty.span() => #ty: ::core::default::Default));
            }
        } else {
            buildable_generics_use.push(set_field_generic_name.clone());
        }
        set_fields_generics.push(field_generic_name);
        all_set.push(set_field_generic_name);
        all_unset.push(unset_field_generic_name);
    }

    // `input.generics.params` contains bounds. Here we get only the params without the bounds for
    // use in type uses, not `impl` declarations.
    let use_struct_generics = input
        .generics
        .params
        .iter()
        .map(|param| match param {
            GenericParam::Type(p) => {
                let ident = &p.ident;
                quote!(#ident,)
            }
            GenericParam::Lifetime(p) => {
                let lt = &p.lifetime;
                quote!(#lt,)
            }
            GenericParam::Const(p) => {
                let ident = &p.ident;
                quote!(#ident,)
            }
        })
        .collect::<proc_macro2::TokenStream>();

    let constrained_generics = quote!(<#struct_generics #(#set_fields_generics),*>);
    let where_clause = &input.generics.where_clause;
    let where_clause = if where_clause.is_some() {
        quote!(#where_clause, #(#default_where_clauses),*)
    } else {
        quote!(where #(#default_where_clauses),*)
    };
    let use_generics = quote!(<#use_struct_generics #(#set_fields_generics),*>);

    // Construct each of the setter methods. These desugar roughly to the following signature:
    //
    //   fn set_<field_name>(self, value: <field_type>) -> <Type>Builder
    //
    let setters = input.fields.iter().enumerate().map(|(i, f)| {
        let (field, method_name) = match &f.ident {
            Some(field) => (quote!(#field), format_ident!("set_{}", field)),
            None => {
                let i = syn::Index::from(i);
                (quote!(#i), format_ident!("set_{}", i))
            }
        };
        let inner_method_name = format_ident!("inner_{}", method_name);
        let decl_generics = set_fields_generics
            .iter()
            .enumerate()
            .filter(|(j, _)| i!=*j)
            .map(|(_, f)| f);
        let decl_generics = quote!(<#struct_generics #(#decl_generics),*>);
        let unset_generics = set_fields_generics
            .iter()
            .zip(input.fields.iter())
            .enumerate()
            .map(|(j, (g, f))| if i == j {
                // FIXME: dedup this logic.
                let field_name = format_ident!("{}", match &f.ident {
                    Some(field) => capitalize(field),
                    None => format!("Field{}", i),
                });
                let f = format_ident!("{}Unset", field_name);
                quote!(#f)
            } else {
                quote!(#g)
            });
        let unset_generics = quote!(<#use_struct_generics #(#unset_generics),*>);
        let set_generics = set_fields_generics
            .iter().zip(input.fields.iter()).enumerate().map(|(j, (g, f))| if i == j {
            let field_name = format_ident!("{}", match &f.ident {
                Some(field) => capitalize(field),
                None => format!("Field{}", i),
            });
            let f = format_ident!("{}Set", field_name);
            quote!(#f)
        } else {
            quote!(#g)
        });
        let set_generics = quote!(<#use_struct_generics #(#set_generics),*>);
        let ty = &f.ty;
        quote! {
            impl #decl_generics #builder_name #unset_generics #where_clause {
                #[must_use]
                pub fn #method_name(mut self, value: #ty) -> #builder_name #set_generics {
                    self.#inner_method_name(value);
                    // We do the following instead of `::core::mem::transmute(self)` here
                    // because we can't `transmute` on fields that involve generics.
                    let ptr = &self as *const #builder_name #unset_generics as *const #builder_name #set_generics;
                    ::core::mem::forget(self);
                    unsafe {
                        ptr.read()
                    }
                }

                fn #inner_method_name(&mut self, value: #ty) {
                    let inner = self.inner.as_mut_ptr();
                    // We know that `inner` is a valid pointer that we can write to.
                    unsafe {
                        ::core::ptr::addr_of_mut!((*inner).#field).write(value);
                    }
                }
            }
        }
    });
    let field_ptr_methods = input.fields.iter().enumerate().map(|(i, f)| {
        let (field, method_name) = match &f.ident {
            Some(field) => (quote!(#field), format_ident!("ptr_{}", i)),
            None => {
                let i = syn::Index::from(i);
                (quote!(#i), format_ident!("ptr_{}", i))
            }
        };
        let ty = &f.ty;
        quote! {
            /// Returns a mutable pointer to a field of the type being built. This is useful if the
            /// initialization requires subtle unsafe shenanigans. You will need to call
            /// `.unsafe_build()` after ensuring all of the fields have been initialized.
            #[must_use]
            pub unsafe fn #method_name(&mut self) -> *mut #ty {
                let inner = self.inner.as_mut_ptr();
                ::core::ptr::addr_of_mut!((*inner).#field)
            }
        }
    });

    let vis = &input.vis;

    let defaults = input.fields.iter().enumerate().filter_map(|(i, f)| {
        let field = match &f.ident {
            Some(field) => format_ident!("inner_set_{}", field),
            None => format_ident!("inner_set_{}", i),
        };
        f.attrs.iter().find_map(|attr| match &attr.meta {
            syn::Meta::Path(path) if path.is_ident("default") => {
                Some(quote!(builder.#field(::core::default::Default::default());))
            }
            syn::Meta::List(syn::MetaList { path, tokens, .. }) if path.is_ident("default") => {
                let mut default_iter = tokens.clone().into_iter();
                let default = match [default_iter.next(), default_iter.next()] {
                    [Some(proc_macro2::TokenTree::Literal(_)), None] => tokens.clone(),
                    _ => syn::Error::new_spanned(tokens, "Expected `#[default(…)]`")
                        .into_compile_error(),
                };
                Some(quote!(builder.#field(#default);))
            }
            _ => None,
        })
    });
    // Construct the params for the `impl` item that provides the `build` method. Normally it would
    // be straightforward: you just specify that all the type params corresponding to fields are
    // set to the `Set` state, but that doesn't account for defaulted type params.
    let build_generics = quote!(<#struct_generics #(#buildable_generics),*>);
    let build_use_generics = quote!(<#use_struct_generics #(#buildable_generics_use),*>);

    let builder_assoc_type = quote! {
        type Builder = #builder_name<#use_struct_generics #(#all_unset),*>;
    };

    let output = quote! {
        #[must_use]
        #[repr(transparent)]
        #vis struct #builder_name #constrained_generics #where_clause {
            inner: ::core::mem::MaybeUninit<#struct_name<#use_struct_generics>>,
            __fields: ::core::marker::PhantomData<(#(#set_fields_generics),*)>,
        }

        #[allow(non_snake_case)]
        #[deny(unused_must_use, clippy::pedantic)]
        mod #mod_name {
            use super::*;

            #(pub struct #all_set;)*
            #(pub struct #all_unset;)*

            impl<#struct_generics> ::makeit::Buildable for #struct_name <#use_struct_generics>
            #where_clause
            {
                #builder_assoc_type

                /// Returns a builder that lets you initialize `Self` field by field in a zero-cost,
                /// type-safe manner.
                #[must_use]
                #[allow(unused_parens)]
                fn builder() -> Self::Builder {
                    let mut builder = #builder_name {
                        inner: unsafe {
                            ::core::mem::MaybeUninit::<Self>::uninit()
                        },
                        __fields: ::core::marker::PhantomData,
                    };
                    #(#defaults)*
                    builder
                }
            }

            impl #build_generics #builder_name #build_use_generics #where_clause {
                /// Finalize the builder.
                #[must_use]
                pub fn build(self) -> #struct_name<#use_struct_generics> {
                    // This method is only callable if all of the fields have been initialized, making
                    // the underlying value at `inner` correctly formed.
                    unsafe { self.unsafe_build() }
                }
            }

            #(#setters)*

            impl #constrained_generics #builder_name #use_generics #where_clause {
                #(#field_ptr_methods)*

                /// HERE BE DRAGONS!
                ///
                /// # Safety
                ///
                /// You're dealing with `MaybeUninit`. If you have to research what that is, you don't
                /// want this.
                #[must_use]
                pub unsafe fn maybe_uninit(self) -> ::core::mem::MaybeUninit<#struct_name<#use_struct_generics>> {
                    self.inner
                }

                /// Only call if you have set a field through their mutable pointer, instead
                /// of using the type-safe builder. It is your responsibility to ensure that
                /// all fields have been set before doing this.
                #[must_use]
                pub unsafe fn unsafe_build(self) -> #struct_name<#use_struct_generics> {
                    self.inner.assume_init()
                }
            }
        }
    };

    TokenStream::from(output.into_token_stream())
}
