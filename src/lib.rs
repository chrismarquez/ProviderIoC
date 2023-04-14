use proc_macro::TokenStream;
use quote::quote;
use syn;

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


#[proc_macro_derive(AutoProvide, attributes(component))]
pub fn provider(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let attribute = &ast.attrs
        .iter()
        .filter(|a| a.path.segments[0].ident == "component")
        .nth(0).unwrap();
    let params: ComponentType = syn::parse2(attribute.tokens.clone()).unwrap();
    let generic = params.0;
    let gen = quote! {
        use std::any::{TypeId};
        use std::ops::Deref;
        use std::sync::Arc;
        use downcast_rs::{DowncastSync, impl_downcast};

        pub trait AutoProvider: Clone {

            type ProviderImpl: Clone;

            fn identity(self) -> Self::ProviderImpl;
            fn store<T: #generic>(&mut self, key: TypeId, value: Arc<T>);
            fn retrieve(&self, key: &TypeId) -> Option<&Arc<dyn #generic>>;

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
    };
    gen.into()
}
