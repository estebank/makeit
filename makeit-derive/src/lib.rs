#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{ItemStruct, Ident, GenericParam, Visibility};

// struct Generics {
//     build_generics_decl: Vec<TokenStream>,
//     build_generics_use: Vec<Ident>,
// }

#[proc_macro_derive(Builder, attributes(default))]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    let input: ItemStruct = syn::parse(input).unwrap();
    let struct_name = &input.ident;
    let builder_name = format_ident!("{}Builder", input.ident);

    let struct_generics = input.generics.params.iter().collect::<Vec<_>>();
    // input.generics.params.iter().map(|param| match param {
    //     GenericParam::Type(p) => {p.ident},
    //     GenericParam::Lifetime(p) => {},
    //     GenericParam::Const(p) => {},
    // });
    let set_fields_generics = input.fields.iter().enumerate().map(|(i, f)| format_ident!("field_{}", match &f.ident {
        Some(field) => field.to_string(),
        None => i.to_string(),
    })).collect::<Vec<_>>();

    let use_struct_generics = input.generics.params.iter().map(|param| match param {
        GenericParam::Type(p) => {
            let ident = &p.ident;
            quote!(#ident)
        }
        GenericParam::Lifetime(p) => {
            let lt = &p.lifetime;
            quote!(#lt)
        }
        GenericParam::Const(p) => {
            let ident = &p.ident;
            quote!(#ident)
        }
    }).collect::<Vec<_>>();

    let comma = if use_struct_generics.is_empty() {
        quote!()
    } else {
        quote!(,)
    };

    let constrained_generics = quote!(<#(#struct_generics),* #comma #(#set_fields_generics),*>);

    let use_generics = quote!(<#(#use_struct_generics),* #comma #(#set_fields_generics),*>);
    // let all_set = input.fields.iter().enumerate().map(|_| format_ident!("field_{}_set"));
    let all_set = input.fields.iter().enumerate().map(|(i, f)| format_ident!("field_{}_set", match &f.ident {
        Some(field) => field.to_string(),
        None => i.to_string(),
    })).collect::<Vec<_>>();
    // let all_unset = set_fields_generics.iter().enumerate().map(|_| format_ident!(::makeit::Unset));
    let all_unset = input.fields.iter().enumerate().map(|(i, f)| format_ident!("field_{}_unset", match &f.ident {
        Some(field) => field.to_string(),
        None => i.to_string(),
    })).collect::<Vec<_>>();
    let setters = input.fields.iter().enumerate().map(|(i, f)| {
        let field = match &f.ident {
            Some(field) => quote!(#field),
            None => quote!(i),
        };
        let method_name = format_ident!("set_{}", match &f.ident {
            Some(field) => field.to_string(),
            None => i.to_string(),
        });
        let inner_method_name = format_ident!("inner_{}", method_name);
        let decl_generics = set_fields_generics.iter().enumerate().filter(|(j, _)| i!=*j).map(|(_, f)| f);
        let decl_generics = quote!(<#(#struct_generics),* #comma #(#decl_generics),*>);
        let unset_generics = set_fields_generics.iter().enumerate().map(|(j, f)| if i == j {
            let f = format_ident!("{}_unset", f);
            quote!(#f)
        } else {
            quote!(#f)
        });
        let unset_generics = quote!(<#(#use_struct_generics),* #comma #(#unset_generics),*>);
        let set_generics = set_fields_generics.iter().enumerate().map(|(j, f)| if i == j {
            let f = format_ident!("{}_set", f);
            quote!(#f)
        } else {
            quote!(#f)
        });
        let set_generics = quote!(<#(#use_struct_generics),* #comma #(#set_generics),*>);
        let ty = &f.ty;
        quote! {
            impl #decl_generics #builder_name #unset_generics {
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
        let field = match &f.ident {
            Some(field) => quote!(#field),
            None => quote!(i),
        };
        let method_name = format_ident!("ptr_{}", match &f.ident {
            Some(field) => field.to_string(),
            None => i.to_string(),
        });
        let ty = &f.ty;
        quote! {
            #[must_use]
            pub unsafe fn #method_name(&mut self) -> *mut #ty {
                let inner = self.inner.as_mut_ptr();
                ::core::ptr::addr_of_mut!((*inner).#field)
            }
        }
    });

    let mod_name = format_ident!("__{}", builder_name);
    let vis = match &input.vis {
        // For private `struct`s we need to change teh visibility of their builders to be
        // accessible from their scope without leaking as `pub`.
        Visibility::Inherited => quote!(pub(super)),
        vis => quote!(#vis),
    };

    let defaults = input.fields.iter().enumerate().filter_map(|(i, f)| {
        let field = match &f.ident {
            Some(field) => format_ident!("inner_set_{}", field),
            None => format_ident!("inner_set_{}", i),
        };
        f.attrs.iter()
            .find(|attr| attr.path.is_ident("default"))
            .map(|attr| {
                let default= &attr.tokens;
                quote!(builder.#field(#default);)
            })
    });
    let build_generics = input.generics.params.iter().collect::<Vec<_>>();
    // let non_default_set = all_set.iter().
    let build_use_generics = quote!(<#(#use_struct_generics),* #comma #(#all_set),*>);
    let input = quote! {
    #[allow(non_snake_case)]
    #[deny(unused_must_use, clippy::pedantic)]
    mod #mod_name {
        use super::#struct_name;

        #[must_use]
        #[repr(transparent)]
        #vis struct #builder_name #constrained_generics {
            inner: ::core::mem::MaybeUninit<#struct_name<#(#use_struct_generics),*>>,
            __fields: ::core::marker::PhantomData<(#(#set_fields_generics),*)>,
        }

        #(pub struct #all_set;)*
        #(pub struct #all_unset;)*

        impl<#(#struct_generics),*> ::makeit::Buildable for #struct_name <#(#use_struct_generics),*> {
            type Builder = #builder_name<#(#use_struct_generics),* #comma #(#all_unset),*>;

            #[allow(unused_parens)]
            #[must_use]
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

        // TODO: account for fields with `default` set in the type params
        impl<#(#build_generics),*> #builder_name #build_use_generics {
            #[must_use]
            pub fn build(self) -> #struct_name<#(#use_struct_generics),*> {
                // This method is only callable if all of the fields have been initialized, making
                // the underlying value at `inner` correctly formed.
                unsafe { self.inner.assume_init() }
            }
        }
        
        #(#setters)*

        impl #constrained_generics #builder_name #use_generics {

            #(#field_ptr_methods)*

            #[must_use]
            pub unsafe fn maybe_uninit(self) -> ::core::mem::MaybeUninit<#struct_name<#(#use_struct_generics),*>> {
                self.inner
            }

            /// Only call if you have set a field through their mutable pointer, instead
            /// of using the type-safe builder. It is your responsibility to ensure that
            /// all fields have been set before doing this.
            #[must_use]
            pub unsafe fn unsafe_build(self) -> #struct_name<#(#use_struct_generics),*> {
                self.inner.assume_init()
            }
        }
    }
    };

    TokenStream::from(input.into_token_stream())
}



// use std::mem::MaybeUninit;
// use std::ptr::addr_of_mut;
// use std::marker::PhantomData;

// struct Role {
//     name: String,
//     disabled: bool,
// }

// struct Set;
// struct Unset;

// struct RoleBuilder<NameSet, DisabledSet> {
//     inner: MaybeUninit<Role>,
//     name: PhantomData<NameSet>,
//     disabled: PhantomData<DisabledSet>,
// }

// impl<A, B> RoleBuilder<A, B> {
//     unsafe fn name_ptr(&mut self) -> *mut String {
//         let role = self.inner.as_mut_ptr();
//         addr_of_mut!((*role).name)
//     }

//     unsafe fn disabled_ptr(&mut self) -> *mut bool {
//         let role = self.inner.as_mut_ptr();
//         addr_of_mut!((*role).disabled)
//     }

//     unsafe fn maybe_uninit(self) -> MaybeUninit<Role> {
//         self.inner
//     }

//     /// Only call if you have set a field through their mutable pointer, instead
//     /// of using the type-safe builder. It is your responsibility to ensure that
//     /// all fields have been set before doing this.
//     unsafe fn unsafe_build(self) -> Role {
//         self.inner.assume_init()
//     }
// }

// impl<A> RoleBuilder<A, Unset> {
//     fn set_disabled(mut self, disabled: bool) -> RoleBuilder<A, Set> {
//         unsafe {
//             let role = self.inner.as_mut_ptr();
//             addr_of_mut!((*role).disabled).write(disabled);
//             std::mem::transmute(self)
//         }
//     }
// }

// impl<B> RoleBuilder<Unset, B> {
//     fn set_name(mut self, name: String) -> RoleBuilder<Set, B> {
//         unsafe {
//             let role = self.inner.as_mut_ptr();
//             addr_of_mut!((*role).name).write(name);
//             std::mem::transmute(self)
//         }
//     }
// }

// impl RoleBuilder<Set, Set> {
//     fn build(mut self) -> Role {
//         unsafe {
//             self.inner.assume_init()
//         }
//     }
// }

// impl Buildable for Role {
//     type Builder = RoleBuilder<Unset, Unset>;
//     fn builder() -> Self::Builder {
//         RoleBuilder {
//             inner: unsafe {
//                 MaybeUninit::<Role>::uninit()
//             },
//             name: PhantomData,
//             disabled: PhantomData,
//         }
//     }
// }

// fn main() {
//     let role = Role::builder()
//         .set_name("basic".to_string())
//         .set_disabled(true)
//         .build();
//     println!("{} ({})", role.name, role.disabled);
    
//     let mut role = Role::builder()
//         .set_name("basic".to_string());
//     let role = unsafe {
//         role.disabled_ptr().write(false);
//         role.unsafe_build()
//     };
//     println!("{} ({})", role.name, role.disabled);
    
//     let role = unsafe {
//         let mut uninit = MaybeUninit::<Role>::uninit();
//         let role = uninit.as_mut_ptr();
//         addr_of_mut!((*role).name).write("basic".to_string());
//         addr_of_mut!((*role).disabled).write(false);
//         uninit.assume_init()
//     };

//     println!("{} ({})", role.name, role.disabled);
// }