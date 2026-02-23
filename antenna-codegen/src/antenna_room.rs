use proc_macro2::TokenStream;
use quote::quote;
use syn::{ImplItem, ItemImpl, Type, parse2};

pub fn antenna_room_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
    // Just passes through the struct for now.
    input
}

pub fn antenna_logic_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item_impl = match parse2::<ItemImpl>(input) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };

    let struct_name = &item_impl.self_ty;

    let mut handlers = Vec::new();
    let mut other_items = Vec::new();

    for item in item_impl.items.drain(..) {
        if let ImplItem::Fn(mut method) = item {
            let mut msg_type = None;
            let mut new_attrs = Vec::new();

            for attr in method.attrs.drain(..) {
                if attr.path().is_ident("msg") {
                    msg_type = Some(
                        attr.parse_args::<Type>()
                            .expect("Expected type in #[msg(...)]"),
                    );
                } else {
                    new_attrs.push(attr);
                }
            }
            method.attrs = new_attrs;

            if let Some(msg_ty) = msg_type {
                let method_name = &method.sig.ident;
                handlers.push(quote! {
                    if let Ok(Packet::User(msg)) = postcard::from_bytes::<Packet<#msg_ty>>(&data) {
                        self.#method_name(ctx, peer_id, msg).await;
                        return;
                    }
                });
                other_items.push(ImplItem::Fn(method));
            } else {
                other_items.push(ImplItem::Fn(method));
            }
        } else {
            other_items.push(item);
        }
    }

    item_impl.items = other_items;

    let mut on_join_call = quote! {};
    let mut on_leave_call = quote! {};

    for item in &item_impl.items {
        if let ImplItem::Fn(method) = item {
            if method.sig.ident == "on_join" {
                on_join_call = quote! { self.on_join(ctx, peer_id).await; };
            }
            if method.sig.ident == "on_leave" {
                on_leave_call = quote! { self.on_leave(ctx, peer_id).await; };
            }
        }
    }

    quote! {
        #item_impl

        #[async_trait::async_trait]
        impl RoomBehavior for #struct_name {
            async fn on_join(&self, ctx: &RoomContext, peer_id: PeerId) {
                #on_join_call
            }

            async fn on_message(&self, ctx: &RoomContext, peer_id: PeerId, data: bytes::Bytes) {
                use postcard::from_bytes;
                use antenna::utils::Packet;

                #(#handlers)*
            }

            async fn on_leave(&self, ctx: &RoomContext, peer_id: PeerId) {
                #on_leave_call
            }
        }
    }
}
