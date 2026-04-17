use wasmtime::component::{Component, Instance, Linker, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime::component::HasSelf;

use wasmtime::component::ResourceAny;
use wasmtime::component::Resource;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
// use wasmtime_wasi::p2::bindings::sync::exports::wasi::cli::run::Guest;
use crate::exports::local::calculator::api::Guest;
use crate::exports::local::calculator::api::Callback;
use crate::local::calculator::callback_handler;

// wasmtime::component::bindgen!("calculator" in "../rquickjs-wasm-lib/wit/calculator.wit");
wasmtime::component::bindgen!({
    world: "calculator",
    path: "../rquickjs-wasm-lib/wit/calculator.wit",
    with: {
        // Adjust this key to match the exact WIT package/interface/resource path.
        // Pattern is: "package:interface/resource-interface.resource"
        "local:calculator/callback-handler.callback": X,
    },
    imports: { default: trappable },
});

pub struct ComponentRunStates {
    // These two are required basically as a standard way to enable the impl of IoView and
    // WasiView.
    // impl of WasiView is required by [`wasmtime_wasi::p2::add_to_linker_sync`]
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
    // You can add other custom host states if needed
}

impl WasiView for ComponentRunStates {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.resource_table,
        }
    }
}

#[repr(C)]
pub struct RuntimeContext;

struct InternalRuntimeContext {
    _engine: Engine,
    store: Store<ComponentRunStates>,
    // instance: Instance,
    calculator: Calculator,
    instance: ResourceAny,
}

#[unsafe(no_mangle)]
pub extern "C" fn init() -> *mut RuntimeContext {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();

    crate::callback_handler::add_to_linker::<ComponentRunStates, HasSelf<_>>(&mut linker, |state: &mut ComponentRunStates| state).unwrap();

    let wasi = WasiCtx::builder().inherit_stdio().inherit_args().build();
    let state = ComponentRunStates {
        wasi_ctx: wasi,
        resource_table: ResourceTable::new(),
    };
    let mut store = Store::new(&engine, state);
    // Instantiate it as a normal component
    let component = Component::from_file(
        &engine,
        "../rquickjs-wasm-lib/target/wasm32-wasip2/release/rquickjs_wasm_lib.wasm",
    )
    .unwrap();
    // let instance = linker.instantiate(&mut store, &component).unwrap();

    let calculator = Calculator::instantiate(&mut store, &component, &linker).unwrap();

    let api = calculator.local_calculator_api();

    let engine_instance = api.engine().call_constructor(&mut store).unwrap();

    let ctx = Box::new(InternalRuntimeContext {
        _engine: engine,
        store,
        calculator,
        instance: engine_instance,
    });

    Box::into_raw(ctx) as *mut RuntimeContext
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(ctx: *mut RuntimeContext, a: u32, b: u32) -> u32 {
    unsafe {
        let mut ctx = Box::from_raw(ctx as *mut InternalRuntimeContext);
        // let result = ctx.calculator.call_add(&mut ctx.store, a, b).unwrap();
        // result
        let api = ctx.calculator.local_calculator_api();
        let result = api
            .engine()
            .call_add(&mut ctx.store, ctx.instance, a, b)
            .unwrap();
        let _ = Box::into_raw(ctx);
        result
    }
}

pub struct X {
    sum: Box<dyn Fn(i32, i32) -> i32 + Send + 'static>,
}

/*
impl callback_handler::HostCallback for X {
    fn get_value(&mut self, _: Resource<Callback>) -> std::result::Result<u32, wasmtime::Error> { Ok(42) }
    fn drop(&mut self, _: Resource<Callback>) -> Result<(), wasmtime::Error> { Ok(())}
}
*/

impl crate::callback_handler::Host for ComponentRunStates {}

impl crate::callback_handler::HostCallback for ComponentRunStates {
    fn get_value(&mut self, cb: Resource<X>) -> wasmtime::Result<u32> {
        let _x: &X = self.resource_table.get(&cb)?;
        let x = (_x.sum)(42, 42);
        Ok(x as u32)
    }

    fn drop(&mut self, cb: Resource<X>) -> wasmtime::Result<()> {
        let _x: X = self.resource_table.delete(cb)?;
        Ok(())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn csharp_to_rust(
    ctx: *mut RuntimeContext,
    cb: extern "C" fn(x: i32, y: i32) -> i32,
) {
    unsafe {
        let mut ctx = Box::from_raw(ctx as *mut InternalRuntimeContext);
        // let result = ctx.calculator.call_add(&mut ctx.store, a, b).unwrap();
        // result
        let api = ctx.calculator.local_calculator_api();

        let x = X { sum: Box::new(move |x, y| cb(x, y)), };
        let res: Resource<callback_handler::Callback> = ctx.store.data_mut().resource_table.push(x).unwrap();
        let result = api
            .engine()
            .call_f(&mut ctx.store, ctx.instance, res)
            .unwrap();
/*
*/
        let _ = Box::into_raw(ctx);
    }
}


/*
impl crate::callback_handler::Host for X {
}
*/


