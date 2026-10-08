use proc_macro2::TokenStream;
use quote::quote;
use syn::*;

pub struct Derive {
    pub ident: Ident,
    pub generics: Generics,
    pub fields: Vec<Field>,
}

// pub struct Variant {
//     pub vis: Visibility,
//     pub name: Ident,
//     // pub fields: Vec<Field>,
// }

// pub struct Field {
//     pub vis: Visibility,
//     pub modifiers: FieldModifiers,
//     pub ident: Option<Ident>,
//     pub ty: Type,
// }

impl Derive {
    pub fn new(input: DeriveInput) -> Result<Self> {
        let syn::Data::Struct(DataStruct { fields, .. }) = input.data else {
            return Err(Error::new_spanned(input, "目前仅支持结构体类型！"));
        };

        let Fields::Named(FieldsNamed { named, .. }) = fields else {
            return Err(Error::new_spanned(fields, "无命名字段的结构体还未实现！"));
        };

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            fields: named.into_iter().collect(),
        })
    }

    pub fn impl_arm_type(&self) -> TokenStream {
        let (impl_generics, type_generics, where_clause) = self.generics.split_for_impl();
        let field_types = self.fields.iter().map(|f| &f.ty).collect::<Vec<_>>();
        let ident = &self.ident;

        let arm_type = quote! { ::anyrch::arm::ArmType };
        let arm_layout = quote! { ::anyrch::arm::ArmLayout };
        let alignup = quote! { ::anyrch::AlignUp };
        quote! {
            impl #impl_generics #arm_type for #ident #type_generics #where_clause {
                const ALIGN: usize = {
                    let mut max_align = 1usize;
                    #(
                        if <#field_types as #arm_type>::ALIGN > max_align {
                            max_align = <#field_types as #arm_type>::ALIGN;
                        }
                    )*
                    max_align
                };

                const SIZE: usize = {

                    let mut offset = 0usize;
                    #(
                        offset = #alignup!(offset, <#field_types as #arm_type>::ALIGN);
                        offset += <#field_types as #arm_layout>::ALIGN_SIZE;
                    )*
                    #alignup!(offset, Self::ALIGN)
                };
            }
        }
    }

    pub fn impl_into_arm(&self) -> TokenStream {
        let into_arm = quote! { ::anyrch::arm::IntoArm };
        let arm_type = quote! { ::anyrch::arm::ArmType };
        let arm_layout = quote! { ::anyrch::arm::ArmLayout };
        let alignup = quote! { ::anyrch::AlignUp };

        let into_fields = self.fields.iter().map(|field| {
            let Field { ident, ty, .. } = field;
            quote! {
                offset = #alignup!(offset, <#ty as #arm_type>::ALIGN);
                #into_arm::into_arm(self.#ident, &mut buf[offset..offset + <#ty as #arm_type>::SIZE]);
                offset += <#ty as #arm_layout>::ALIGN_SIZE;
            }
        });

        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let ident = &self.ident;
        quote! {
            impl #impl_generics #into_arm for #ident #ty_generics #where_clause {
                #[allow(unused)]
                fn into_arm(self, buf: &mut [u8]) {
                    debug_assert!(buf.len() >= <Self as #arm_type>::SIZE);
                    let mut offset = 0;
                    #( #into_fields )*
                }
            }
        }
    }

    pub fn impl_from_arm(&self) -> TokenStream {
        let from_arm = quote! { ::anyrch::arm::FromArm };
        let arm_type = quote! { ::anyrch::arm::ArmType };
        let arm_layout = quote! { ::anyrch::arm::ArmLayout };
        let alignup = quote! { ::anyrch::AlignUp };

        let field_idents = self.fields.iter().map(|f| f.ident.as_ref());
        let from_fields = self.fields.iter().map(|field| {
            let Field { ident, ty, .. } = field;
            quote! {
                offset = #alignup!(offset, <#ty as #arm_type>::ALIGN);
                let #ident = #from_arm::from_arm(arm, &buf[offset..offset + <#ty as #arm_type>::SIZE]);
                offset += <#ty as #arm_layout>::ALIGN_SIZE;
            }
        });

        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let ident = &self.ident;
        quote! {
            impl #impl_generics #from_arm for #ident #ty_generics #where_clause {
                #[allow(unused)]
                fn from_arm(arm: &Arm, buf: &[u8]) -> Self {
                    debug_assert!(buf.len() >= <Self as #arm_type>::SIZE);
                    let mut offset = 0;
                    #( #from_fields )*
                    Self { #( #field_idents, )* }
                }
            }
        }
    }
}
