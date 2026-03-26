use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, parse2};

struct AntennaClientArgs {
    _msg_type: Ident,
}

impl Parse for AntennaClientArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let msg_type: Ident = input.parse()?;
        Ok(AntennaClientArgs {
            _msg_type: msg_type,
        })
    }
}

pub fn antenna_client_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let AntennaClientArgs { _msg_type } = match parse2::<AntennaClientArgs>(args) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error(),
    };

    let item_struct = match parse2::<syn::ItemStruct>(input) {
        Ok(s) => s,
        Err(e) => return e.to_compile_error(),
    };

    let struct_name = &item_struct.ident;

    quote! {
        #item_struct

        #[wasm_bindgen]
        impl #struct_name {
            pub fn on_event(&self, cb: js_sys::Function) {
                self.engine.set_event_handler(cb);
            }
        }
    }
}
