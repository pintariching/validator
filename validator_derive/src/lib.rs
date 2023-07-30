// #![recursion_limit = "128"]
#![allow(unused)]

use darling::ast::Data;
use darling::util::Override;
use darling::{FromDeriveInput, FromField, FromMeta};
use proc_macro_error::proc_macro_error;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, DeriveInput, Expr};

use tokens::cards::credit_card_tokens;
use tokens::email::email_tokens;
use tokens::length::length_tokens;
use types::*;

mod tokens;
mod types;
mod utils;

// This struct holds all the validation information on a field
// The "ident" and "ty" fields are populated by `darling`
// The others are our attributes for example:
// #[validate(email(message = "asdfg"))]
//            ^^^^^
//
#[derive(Debug, FromField)]
#[darling(attributes(validate))]
struct ValidateField {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    credit_card: Option<Override<Card>>,
    contains: Option<Contains>,
    does_not_contain: Option<DoesNotContain>,
    email: Option<Override<Email>>,
    ip: Option<Ip>,
    length: Option<Length>,
    must_match: Option<MustMatch>,
    non_control_character: Option<NonControlCharacter>,
    range: Option<Range>,
    required: Option<Required>,
    url: Option<Url>,
}

// The field gets converted to tokens in the same format as it was before
impl ToTokens for ValidateField {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let field_name = self.ident.clone().unwrap();
        let field_name_str = self.ident.clone().unwrap().to_string();

        // Length validation
        let length = if let Some(length) = self.length.clone() {
            length_tokens(length, &field_name, &field_name_str)
        } else {
            quote!()
        };

        // Email validation
        let email = if let Some(email) = self.email.clone() {
            email_tokens(
                match email {
                    Override::Inherit => Email::default(),
                    Override::Explicit(e) => e,
                },
                &field_name,
                &field_name_str,
            )
        } else {
            quote!()
        };

        let card = if let Some(credit_card) = self.credit_card.clone() {
            credit_card_tokens(
                match credit_card {
                    Override::Inherit => Card::default(),
                    Override::Explicit(c) => c,
                },
                &field_name,
                &field_name_str,
            )
        } else {
            quote!()
        };

        tokens.extend(quote! {
            #length
            #email
            #card
        });
    }
}

// The main struct we get from parsing the attributes
// The "supports(struct_named)" should guarantee to only have this
// macro work with structs with named fields I think?
#[derive(FromDeriveInput)]
#[darling(attributes(validate), supports(struct_named))]
struct ValidationData {
    ident: syn::Ident,
    generics: syn::Generics,
    data: Data<(), ValidateField>,
}

#[proc_macro_derive(Validate, attributes(validate))]
#[proc_macro_error]
pub fn derive_validation(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: DeriveInput = parse_macro_input!(input);

    // Parse the input to the ValidationData struct defined above
    let validation_data = match ValidationData::from_derive_input(&input) {
        Ok(data) => data,
        Err(e) => return e.write_errors().into(),
    };

    // Get all the fields to quote them below
    let validation_field = validation_data.data.take_struct().unwrap().fields;

    let ident = validation_data.ident;
    let (imp, ty, gen) = validation_data.generics.split_for_impl();

    quote! {
        impl #imp ::validator::Validate for #ident #ty #gen {
            fn validate (&self) -> ::std::result::Result<(), ::validator::ValidationErrors> {
                let mut errors = ::validator::ValidationErrors::new();

                #(#validation_field)*

                if errors.is_empty() {
                    ::std::result::Result::Ok(())
                } else {
                    ::std::result::Result::Err(errors)
                }
            }
        }
    }
    .into()
}
