use proc_macro::TokenStream;

mod antenna_client;
mod antenna_client_v2;
mod antenna_room;

#[proc_macro_attribute]
pub fn antenna_client(args: TokenStream, input: TokenStream) -> TokenStream {
    antenna_client::antenna_client_impl(args.into(), input.into()).into()
}

#[proc_macro_attribute]
pub fn antenna_client_v2(args: TokenStream, input: TokenStream) -> TokenStream {
    antenna_client_v2::antenna_client_impl(args.into(), input.into()).into()
}

#[proc_macro_attribute]
pub fn antenna_room(args: TokenStream, input: TokenStream) -> TokenStream {
    antenna_room::antenna_room_impl(args.into(), input.into()).into()
}

#[proc_macro_attribute]
pub fn antenna_logic(args: TokenStream, input: TokenStream) -> TokenStream {
    antenna_room::antenna_logic_impl(args.into(), input.into()).into()
}
