use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use quote::{ToTokens, quote};
use syn::{FnArg, ItemFn, ReturnType, Type, parse_macro_input};

#[proc_macro_error]
#[proc_macro_attribute]
pub fn yields(tokens: TokenStream, item: TokenStream) -> TokenStream {
    let yield_ty = if tokens.is_empty() {
        quote! { () }
    } else {
        parse_macro_input!(tokens as Type).into_token_stream()
    };
    let func = parse_macro_input!(item as ItemFn);
    let args = func.sig.inputs.clone().into_iter().collect::<Vec<FnArg>>();
    let (arg_ty, arg_pat_ty) = match args.as_slice() {
        [] => (quote! { () }, None),
        [FnArg::Typed(pat_type)] => (
            pat_type.ty.clone().into_token_stream(),
            Some(pat_type.into_token_stream()),
        ),
        _ => {
            abort!(
                func.sig.inputs,
                "incorrect arguments, at most 1 argument is allowed"
            )
        }
    };
    let return_ty = match func.sig.output.clone() {
        ReturnType::Default => quote! { () }.into_token_stream(),
        ReturnType::Type(_, ty) => ty.into_token_stream(),
    };
    let vis = func.vis;
    let struct_ident = func.sig.ident;
    let block = func.block;
    quote! {
        #vis struct #struct_ident(<Self as ::coro::Coro>::Coro);
        // #[rust_analyzer::skip]
        impl ::coro::Coro for #struct_ident {
            type Arg = #arg_ty;
            type Coro = impl ::core::ops::Coroutine<
                #arg_ty,
                Yield = #yield_ty,
                Return = #return_ty
            >;
            #[inline]
            fn new_coro() -> Self::Coro {
                #[coroutine] |#arg_pat_ty| #block
            }
        }
        impl ::core::ops::Coroutine<#arg_ty> for #struct_ident {
            type Yield = #yield_ty;
            type Return = #return_ty;
            #[inline]
            fn resume(
                self: ::core::pin::Pin<&mut Self>,
                arg: #arg_ty,
            ) -> ::core::ops::CoroutineState<#yield_ty, #return_ty> {
                // Safety: This is okay because `self.0` is pinned when `self` is.
                unsafe { self.map_unchecked_mut(|s| &mut s.0) }.resume(arg)
            }
        }
        impl #struct_ident {
            #[inline]
            fn new() -> Self {
                Self(<Self as ::coro::Coro>::new_coro())
            }
        }
        impl ::core::default::Default for #struct_ident {
            #[inline]
            fn default() -> Self {
                Self::new()
            }
        }
    }
    .into()
}

// quote! {
//     struct #struct_ident(::core::UnsafeCell<::core::Option<<Self as ::coro::CoroGen>::Coro>>);
//     impl #struct_ident {
//         pub const fn new() -> Self {
//             Self(UnsafeCell::new(None))
//         }
//         pub unsafe fn init(&'static self) {
//             unsafe {
//                 self.0
//                     .as_mut_unchecked()
//                     .insert(Self::new_coro())
//             };
//         }
//         pub unsafe fn as_mut(
//             &'static self,
//         ) -> &'static mut <#struct_ident as ::coro::CoroGen>::Coro {
//             unsafe { self.0.as_mut_unchecked().as_mut().unwrap_unchecked() }
//         }
//     }
//     impl ::coro::CoroGen for #struct_ident {
//         type Coro = impl Coroutine<(), Yield = (), Return = ()>;
//         fn new_coro() -> Self::Coro {
//         }
//     }
//     unsafe impl Sync for #struct_ident {}
//     #func_sig {
//         #[coroutine]
//         || {
//             #block
//         }
//     }
// }
// .into()
