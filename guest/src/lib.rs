#![allow(unsafe_op_in_unsafe_fn)]

use crate::bindings::exports::rquickjs::wasm::engine_api;
use crate::bindings::exports::rquickjs::wasm::engine_api::Guest;
use crate::bindings::rquickjs::wasm::callback_api;
use crate::bindings::rquickjs::wasm::callback_api::Param;
use callback_api::CallbackError;
use rquickjs::function::Rest;
use rquickjs::{Array, Context, Error, Function, Runtime, Value};

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

impl From<Error> for CallbackError {
    fn from(value: Error) -> Self {
        CallbackError {
            message: value.to_string(),
            error_code: callback_api::ErrorCode::Rquickjs,
        }
    }
}

impl bindings::exports::rquickjs::wasm::engine_api::GuestEngine for Engine {
    fn create() -> Result<engine_api::Engine, callback_api::CallbackError> {
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;
        Ok(engine_api::Engine::new(Engine {
            ctx: context,
            _rt: runtime,
        }))
    }

    fn eval(&self, script: String) -> Result<Param, CallbackError> {
        self.ctx.with(|ctx| -> Result<Param, CallbackError> {
            let value = ctx.eval::<Value, _>(script)?;
            if value.is_null() {
                Ok(Param::Null)
            } else if value.is_int() {
                Ok(Param::Int(Some(value.get()?)))
            } else if value.is_string() {
                Ok(Param::Str(Some(value.get()?)))
            } else if value.is_undefined() {
                Ok(Param::Unit)
            } else if value.is_array() {
                Err(CallbackError {
                    message: "is_array not implemented.".to_string(),
                    error_code: callback_api::ErrorCode::Todo,
                })
            } else {
                Err(CallbackError {
                    message: "woot?".to_string(),
                    error_code: callback_api::ErrorCode::Eval,
                })
            }
        })
    }

    fn register(
        &self,
        name: String,
        callback: callback_api::Callback,
    ) -> Result<(), CallbackError> {
        self.ctx.with(|ctx| -> Result<(), CallbackError> {
            let callback = callback;
            let global = ctx.globals();
            let name_cloned = name.clone();

            _ = global.set(
                &name.clone(),
                Function::new(ctx.clone(), move |params: Rest<Value>| {
                    let params: Vec<Param> = params
                        .0
                        .into_iter()
                        .map(|v| {
                            if v.is_null() {
                                Param::Null
                            } else if v.is_int() {
                                Param::Int(Some(v.as_int().expect("Just verified that it is int.")))
                            } else if v.is_array() {
                                let array: Vec<callback_api::LazyParam> = v
                                    .as_array()
                                    .iter()
                                    .map(|i| {
                                        if i.is_int() {
                                            callback_api::LazyParam::new(Param::Int(Some(
                                                i.as_int().expect("Just verified that it is int."),
                                            )))
                                        } else {
                                            todo!("recursive!")
                                        }
                                    })
                                    .collect();

                                Param::Vec(Some(array))
                            } else {
                                todo!("")
                            }
                        })
                        .collect();

                    match callback.invoke(&name, params) {
                        Param::Unit => Value::new_undefined(ctx.clone()),
                        Param::Vec(Some(result)) => {
                            let array = Array::new(ctx.clone()).expect("Couldn't create Array");
                            for (idx, item) in result.into_iter().enumerate() {
                                let item = match item.get() {
                                    Param::Int(Some(i)) => Value::new_int(ctx.clone(), i),
                                    _ => todo!(),
                                };
                                array.set(idx, item).expect("Couldn't set item in Array");
                            }
                            Value::from_array(array)
                        }
                        Param::Int(Some(result)) => Value::new_int(ctx.clone(), result),
                        Param::Str(Some(result)) => Value::from_string(
                            rquickjs::String::from_str(ctx.clone(), &result)
                                .expect("Should be able to create string"),
                        ),
                        Param::Vec(None) | Param::Str(None) | Param::Int(None) | Param::Null => {
                            Value::new_null(ctx.clone())
                        }
                    }
                })?
                .with_name(&name_cloned)?,
            )?;

            Ok(())
        })?;
        Ok(())
    }
}

struct ContextHost;

impl Guest for ContextHost {
    type Engine = Engine;
}

bindings::export!(ContextHost with_types_in bindings);
