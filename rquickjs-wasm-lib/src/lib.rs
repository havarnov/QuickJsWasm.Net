#![allow(unsafe_op_in_unsafe_fn)]

use rquickjs::{Context, Function, Runtime, Value, Array};
use rquickjs::function::Rest;

use crate::bindings::exports::rquickjs::wasm::engine_api::Guest;
use crate::bindings::rquickjs::wasm::callback_api;

#[allow(unused)]
mod bindings {
    wit_bindgen::generate!({
        world: "rquickjs",
    });
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
        self.ctx
            .with(|ctx| -> Result<(), ()> {
                let callback = callback;
                let global = ctx.globals();

                _ = global.set(
                    &name,
                    Function::new(ctx.clone(), move |params: Rest<Value>| {
                        let params: Vec<callback_api::Param> = params
                         .0
                         .into_iter()
                         .map(|v| {
                             if v.is_int() {
                                 callback_api::Param::Int(v.as_int().unwrap())
                             } else if v.is_array() {
                                 let array: Vec<callback_api::LazyParam> = v.as_array().iter()
                                     .map(|i| {
                                         if i.is_int() {
                                            callback_api::LazyParam::new(callback_api::Param::Int(i.as_int().unwrap()))
                                         }
                                         else {
                                             todo!()
                                         }
                                     })
                                     .collect();

                                 callback_api::Param::Vec(array)
                             } else {
                                 todo!("")
                             }
                         })
                         .collect();

                        match callback.invoke(params) {
                            callback_api::Param::Unit => Value::new_undefined(ctx.clone()),
                            callback_api::Param::Vec(result) => {
                                let array = Array::new(ctx.clone()).unwrap();
                                for (idx, item) in result.into_iter().enumerate()
                                {
                                    let item = match item.get() {
                                        callback_api::Param::Int(i) => Value::new_int(ctx.clone(), i),
                                        _ => todo!(),
                                    };
                                    array.set(idx, item).unwrap();
                                }
                                Value::from_array(array)
                            },
                            callback_api::Param::Int(result) => {
                                Value::new_int(ctx.clone(), result)
                            }
                        }
                    })
                    .unwrap()
                    .with_name(&name)
                    .unwrap(),
                );

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
