use rquickjs::{Context, Function, Runtime};

use crate::bindings::exports::rquickjs::wasm::engine_api::Guest;
use crate::bindings::rquickjs::wasm::callbacks;

#[allow(unused)]
mod bindings {
    wit_bindgen::generate!({ world: "rquickjs" });
}

struct Engine {
    ctx: Context,
    _rt: Runtime,
}

// 1. Implement the trait for the resource inside the interface
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

    fn register(&self, name: String, func: callbacks::FUnitUnit) {
        self.ctx
            .with(|ctx| -> Result<(), ()> {
                let func = func;
                let global = ctx.globals();

                _ = global.set(
                    &name,
                    Function::new(ctx.clone(), move || func.get_implementation())
                        .unwrap()
                        .with_name(&name)
                        .unwrap(),
                );
                Ok(())
            })
            .unwrap();
    }
}

// 2. Define the provider struct for the export macro
struct ContextHost;

impl Guest for ContextHost {
    type Engine = Engine;
}

// 3. The export macro now maps the WIT 'api' to our 'Engine' implementation.
// Note: We do NOT need "impl bindings::Guest for ContextHost" here.
bindings::export!(ContextHost with_types_in bindings);
