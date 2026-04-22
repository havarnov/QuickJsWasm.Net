#![allow(unsafe_op_in_unsafe_fn)]

use rquickjs::{Context, Function, Runtime, Value};

use crate::bindings::exports::rquickjs::wasm::engine_api::Guest;
use crate::bindings::rquickjs::wasm::callback_api;

#[allow(unused)]
mod bindings {
    wit_bindgen::generate!({ world: "rquickjs" });
}

struct Engine {
    ctx: Context,
    _rt: Runtime,
}

impl bindings::exports::rquickjs::wasm::engine_api::GuestEngine for Engine {
    fn new() -> Self {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        Engine {
            ctx: context,
            _rt: runtime,
        }
    }

    fn eval(&self, script: String) {
        self.ctx
            .with(|ctx| -> Result<(), ()> {
                ctx.eval::<(), _>(script).unwrap();
                Ok(())
            })
            .unwrap();
    }

    fn register(&self, name: String, callback: callback_api::Callback) {
        let params = callback.params();
        self.ctx
            .with(|ctx| -> Result<(), ()> {
                let callback = callback;
                let global = ctx.globals();

                match params.as_slice() {
                    [callback_api::ParamType::Uint] => {
                        _ = global.set(
                            &name,
                            Function::new(ctx.clone(), move |i: u32| {
                                match callback.invoke(&vec![callback_api::Param::Uint(i)]) {
                                    callback_api::Param::Unit => Value::new_undefined(ctx.clone()),
                                    callback_api::Param::Uint(result) => {
                                        Value::new_int(ctx.clone(), result as i32)
                                    }
                                }
                            })
                            .unwrap()
                            .with_name(&name)
                            .unwrap(),
                        );
                    }
                    [callback_api::ParamType::Unit] => {
                        _ = global.set(
                            &name,
                            Function::new(ctx.clone(), move || _ = callback.invoke(&vec![]))
                                .unwrap()
                                .with_name(&name)
                                .unwrap(),
                        );
                    }
                    _ => todo!(),
                };
                /*
                _ = global.set(
                    &name,
                    Function::new(ctx.clone(), move || _ = callback.invoke(&vec![]))
                        .unwrap()
                        .with_name(&name)
                        .unwrap(),
                );
                */
                Ok(())
            })
            .unwrap();
    }
}

struct ContextHost;

impl Guest for ContextHost {
    type Engine = Engine;
}

bindings::export!(ContextHost with_types_in bindings);
