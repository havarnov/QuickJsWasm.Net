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
        let runtime = Runtime::new().expect("her1");
        let context = Context::full(&runtime).expect("her2");
        Engine {
            ctx: context,
            _rt: runtime,
        }
    }

    fn eval(&self, script: String) {
        self.ctx
            .with(|ctx| -> Result<(), ()> {
                ctx.eval::<(), _>(script).expect("her3");
                Ok(())
            })
            .expect("her4");
    }

    fn register(&self, name: String, callback: callback_api::Callback) {
        self.ctx
            .with(|ctx| -> Result<(), ()> {
                let callback = callback;
                let global = ctx.globals();
                let name_cloned = name.clone();

                _ = global.set(
                    &name.clone(),
                    Function::new(ctx.clone(), move |params: Rest<Value>| {
                        let params: Vec<callback_api::Param> = params
                         .0
                         .into_iter()
                         .map(|v| {
                             if v.is_int() {
                                 callback_api::Param::Int(v.as_int().expect("her5"))
                             } else if v.is_array() {
                                 let array: Vec<callback_api::LazyParam> = v.as_array().iter()
                                     .map(|i| {
                                         if i.is_int() {
                                            callback_api::LazyParam::new(callback_api::Param::Int(i.as_int().expect("her6")))
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


                        match callback.invoke(&name, params) {
                            callback_api::Param::Unit => Value::new_undefined(ctx.clone()),
                            callback_api::Param::Vec(result) => {
                                let array = Array::new(ctx.clone()).expect("her7");
                                for (idx, item) in result.into_iter().enumerate()
                                {
                                    let item = match item.get() {
                                        callback_api::Param::Int(i) => Value::new_int(ctx.clone(), i),
                                        _ => todo!(),
                                    };
                                    array.set(idx, item).expect("her8");
                                }
                                Value::from_array(array)
                            },
                            callback_api::Param::Int(result) => {
                                Value::new_int(ctx.clone(), result)
                            }
                        }
                    })
                    .expect("her9")
                    .with_name(&name_cloned)
                    .expect("her10"),
                );

                Ok(())
            })
            .expect("her11");
    }
}

struct ContextHost;

impl Guest for ContextHost {
    type Engine = Engine;
}

bindings::export!(ContextHost with_types_in bindings);
