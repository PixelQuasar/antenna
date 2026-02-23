use proc_macro::TokenStream;

mod antenna_client;
mod antenna_room;

#[proc_macro_attribute]
pub fn antenna_client(args: TokenStream, input: TokenStream) -> TokenStream {
    antenna_client::antenna_client_impl(args.into(), input.into()).into()
}

#[proc_macro_attribute]
pub fn antenna_room(args: TokenStream, input: TokenStream) -> TokenStream {
    antenna_room::antenna_room_impl(args.into(), input.into()).into()
}

#[proc_macro_attribute]
pub fn antenna_logic(args: TokenStream, input: TokenStream) -> TokenStream {
    antenna_room::antenna_logic_impl(args.into(), input.into()).into()
}
