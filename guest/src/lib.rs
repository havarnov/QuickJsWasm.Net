#![allow(unsafe_op_in_unsafe_fn)]

use crate::bindings::exports::rquickjs::wasm::engine_api;
use crate::bindings::exports::rquickjs::wasm::engine_api::Guest;
use crate::bindings::rquickjs::wasm::callback_api;
use crate::bindings::rquickjs::wasm::callback_api::Param;
use callback_api::CallbackError;
use rquickjs::function::Rest;
use rquickjs::{Array, Context, Ctx, Error, Function, Runtime, Value};

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
            Ok(value.into())
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

            let func = Function::new(ctx.clone(), move |params: Rest<Value>| {
                let params: Vec<Param> = params.0.into_iter().map(|v| v.into()).collect();
                callback.invoke(&name_cloned, params).into_value(ctx.clone())
            })?
            .with_name(&name)?;

            global.set(&name.clone(), func)?;

            Ok(())
        })?;
        Ok(())
    }
}

impl Param {
    fn into_value(self, ctx: Ctx) -> Value {
        match self {
            Param::Int(Some(i)) => Value::new_int(ctx.clone(), i),
            Param::Str(Some(s)) => Value::from_string(
                rquickjs::String::from_str(ctx.clone(), &s).expect("Should be able to create string")),
            Param::Unit => Value::new_undefined(ctx.clone()),
            Param::Vec(None) | Param::Str(None) | Param::Int(None) | Param::Null => {
                Value::new_null(ctx.clone())
            }
            Param::Vec(Some(result)) => {
                let array = Array::new(ctx.clone()).expect("Couldn't create Array");
                for (idx, item) in result.into_iter().enumerate() {
                    let item = item.get().into_value(ctx.clone());
                    array.set(idx, item).expect("Couldn't set item in Array");
                }
                Value::from_array(array)
            }
        }
    }
}

impl Into<Param> for Value<'_> {
    fn into(self) -> Param {
        if self.is_null() {
            Param::Null
        } else if self.is_int() {
            Param::Int(Some(self.as_int().expect("Just verified that it is int.")))
        } else if self.is_string() {
            Param::Str(Some(
                self.as_string()
                    .expect("Just verified that it is string.")
                    .to_string()
                    .expect("Should be able to create string"),
            ))
        } else if self.is_undefined() {
            Param::Unit
        } else if self.is_array() {
            Param::Vec(Some(
                self.as_array()
                    .iter()
                    .map(|item| {
                        let item: Value = item.as_value().clone();
                        let item: Param = item.into();
                        callback_api::LazyParam::new(item)
                    })
                    .collect(),
            ))
        } else {
            todo!("NOT IMPLEMENTED VALUE?")
        }
    }
}

struct ContextHost;

impl Guest for ContextHost {
    type Engine = Engine;
}

bindings::export!(ContextHost with_types_in bindings);
