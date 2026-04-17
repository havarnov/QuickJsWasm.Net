use rquickjs::{Context, Function, Runtime};

use crate::bindings::exports::local::calculator::api::Guest;
use crate::bindings::local::calculator::callback_handler;

#[allow(unused)]
mod bindings {
    wit_bindgen::generate!({ world: "calculator" });
}

struct Engine {
    ctx: Context,
    _rt: Runtime,
}

// 1. Implement the trait for the resource inside the interface
impl bindings::exports::local::calculator::api::GuestEngine for Engine {
    fn new() -> Self {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        Engine {
            ctx: context,
            _rt: runtime,
        }
    }

    fn add(&self, x: u32, y: u32) -> u32 {
        self.ctx.with(|ctx| {
            ctx.eval::<u32, _>(format!(
                "var foo = (foo == undefined ? 0 : foo) + {x} + {y} + __print({x}, {y}); foo"
            ))
            .unwrap()
        })
    }

    fn f(&self, cb: callback_handler::Callback) -> u32 {
        self.ctx
            .with(|ctx| -> Result<(), ()> {
                let cb = cb;
                let global = ctx.globals();

                _ = global.set(
                    "__print",
                    Function::new(ctx.clone(), move || cb.get_value())
                        .unwrap()
                        .with_name("__print")
                        .unwrap(),
                );
                Ok(())
            })
            .unwrap();

        32
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
