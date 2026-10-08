use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[cfg(feature = "arm")]
mod arm;

#[cfg(feature = "arm")]
#[proc_macro_derive(Arm)]
pub fn derive_arm(input: TokenStream) -> TokenStream {
    match arm::Derive::new(parse_macro_input!(input)) {
        Ok(derive) => {
            let arm_type = derive.impl_arm_type();
            let into_arm = derive.impl_into_arm();
            let from_arm = derive.impl_from_arm();
            TokenStream::from(quote! { #arm_type #into_arm #from_arm })
        },
        Err(e) => e.to_compile_error().into(),
    }
}

#[cfg(feature = "arm")]
#[proc_macro_derive(ArmType)]
pub fn derive_arm_type(input: TokenStream) -> TokenStream {
    match arm::Derive::new(parse_macro_input!(input)) {
        Ok(derive) => TokenStream::from(derive.impl_arm_type()),
        Err(e) => e.to_compile_error().into(),
    }
}

#[cfg(feature = "arm")]
#[proc_macro_derive(IntoArm)]
pub fn derive_into_arm(input: TokenStream) -> TokenStream {
    match arm::Derive::new(parse_macro_input!(input)) {
        Ok(derive) => {
            let token = derive.impl_into_arm();
            println!("{}", token);
            TokenStream::from(token)
        }
        Err(e) => e.to_compile_error().into(),
    }
}

#[cfg(feature = "arm")]
#[proc_macro_derive(FromArm)]
pub fn derive_from_arm(input: TokenStream) -> TokenStream {
    match arm::Derive::new(parse_macro_input!(input)) {
        Ok(derive) => TokenStream::from(derive.impl_from_arm()),
        Err(e) => e.to_compile_error().into(),
    }
}
