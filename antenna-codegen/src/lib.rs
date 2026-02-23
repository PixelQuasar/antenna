use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Token, parse_macro_input};

struct AntennaClientArgs {
    client_msg: Ident,
    server_msg: Ident,
}

impl Parse for AntennaClientArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let client_msg: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let server_msg: Ident = input.parse()?;
        Ok(AntennaClientArgs {
            client_msg,
            server_msg,
        })
    }
}

#[proc_macro_attribute]
pub fn antenna_client(args: TokenStream, input: TokenStream) -> TokenStream {
    let AntennaClientArgs {
        client_msg,
        server_msg,
    } = parse_macro_input!(args as AntennaClientArgs);

    let item_struct = parse_macro_input!(input as syn::ItemStruct);
    let struct_name = &item_struct.ident;

    let server_msg_str = server_msg.to_string();
    let callback_type_str = format!("(event: {}) => void", server_msg_str);
    let import_str = format!(
        "import {{ {} }} from './types/{}';",
        server_msg_str, server_msg_str
    );

    let callback_ident = Ident::new(&format!("{}Callback", struct_name), struct_name.span());
    let import_const_ident = Ident::new(
        &format!("_TS_IMPORT_{}", struct_name).to_uppercase(),
        struct_name.span(),
    );

    let expanded = quote! {
        #item_struct

        #[wasm_bindgen(typescript_custom_section)]
        const #import_const_ident: &'static str = #import_str;

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(typescript_type = #callback_type_str)]
            pub type #callback_ident;
        }

        #[wasm_bindgen]
        impl #struct_name {
            pub fn on_event(&self, cb: #callback_ident) {
                use wasm_bindgen::JsCast;
                let func: js_sys::Function = cb.unchecked_into();
                self.engine.set_event_handler(func);
            }

            pub fn on_track(&self, cb: js_sys::Function) {
                self.engine.set_track_handler(cb);
            }

            pub fn add_track(&self, track: web_sys::MediaStreamTrack, stream: web_sys::MediaStream) -> Result<(), wasm_bindgen::JsValue> {
                self.engine.add_track(track, stream)
            }
        }
    };

    TokenStream::from(expanded)
}
