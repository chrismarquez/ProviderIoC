use proc_macro::TokenStream;
use quote::quote;
use syn;
use syn::DeriveInput;
use syn::__private::Span;
use syn::parse::Parser;
use syn::parse_macro_input;



/// Data struct used to extract a Generic type identifier
struct ComponentType(syn::Ident);
impl syn::parse::Parse for ComponentType {
    /// The implementation for the Parse trait that allows building the struct from a ParseStream
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input); // Extracts the Input segment between parentheses
        let generic: syn::Ident = content.parse()?;
        Ok(ComponentType(generic))
    }
}


#[proc_macro_attribute]
pub fn enable_auto_provide(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let attribute = &ast.attrs
        .iter()
        .filter(|a| a.path.segments[0].ident == "component")
        .nth(0).unwrap();

    let params: ComponentType = syn::parse2(attribute.tokens.clone()).unwrap();
    let generic = params.0;
    let new_field = quote! { pub set: HashMap<TypeId, Arc<dyn #generic>> };
    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => {
            match &mut struct_data.fields {
                syn::Fields::Named(fields) => {
                    fields.named
                        .push(syn::Field::parse_named.parse2(new_field).unwrap());
                }
                _ => {
                    ()
                }
            }

            return quote! {
                impl_downcast!(sync #generic);
                pub trait #generic: DowncastSync {}

                #[derive(AutoProvide, Clone)]
                #ast
            }.into();
        }
        _ => panic!("`add_field` has to be used with structs "),
    }
}


#[proc_macro_derive(AutoProvide, attributes(component))]
pub fn provider(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let struct_name = &ast.ident;
    let attribute = &ast.attrs
        .iter()
        .filter(|a| a.path.segments[0].ident == "component")
        .nth(0).unwrap();

    let params: ComponentType = syn::parse2(attribute.tokens.clone()).unwrap();
    let generic = params.0;
    let generic_trait = syn::Ident::new(&format!("{}AutoProvider", generic), Span::call_site());
    let gen = quote! {
        use std::any::{TypeId};
        use std::ops::Deref;
        use std::sync::Arc;
        use downcast_rs::{DowncastSync, impl_downcast};

        pub trait #generic_trait: Clone {
            type ProviderImpl: Clone;
            fn identity(self) -> Self::ProviderImpl;
            fn store<T: #generic>(&mut self, key: TypeId, value: Arc<T>);
            fn retrieve(&self, key: &TypeId) -> Option<&Arc<dyn #generic>>;

            fn base() -> #struct_name {
                #struct_name { set: HashMap::new() }
            }

            fn manage<T: #generic>(&mut self, item: T) -> Self::ProviderImpl  {
                 self.store(item.type_id(), Arc::new(item));
                 self.deref().clone().identity()
            }

            fn get<U: #generic>(&self) -> Arc<U> {
                let key = TypeId::of::<U>();
                if let Some(it) = self.retrieve(&key) {
                    if let Ok(service) = it.clone().downcast_arc::<U>()  {
                        return service.clone()
                    }
                }
                panic!("Service of type {} not found. Did you forget a manage() call?", std::any::type_name::<U>())
            }
        }

        impl #struct_name {

        }

        impl #generic_trait for #struct_name {
            type ProviderImpl = Self;
            fn identity(self) -> Self::ProviderImpl { self }
            fn store<T: #generic>(&mut self, key: TypeId, value: Arc<T>) {
                self.set.insert(key, value);
            }
            fn retrieve(&self, key: &TypeId) -> Option<&Arc<dyn #generic>> {
                self.set.get(key)
            }
        }

    };
    gen.into()
}
