use wasmtime::component::{Component, Instance, Linker, ResourceTable}; use wasmtime::{Engine, Store};

use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
use wasmtime::component::ResourceAny;
// use wasmtime_wasi::p2::bindings::sync::exports::wasi::cli::run::Guest;
use crate::exports::local::calculator::api::Guest;

wasmtime::component::bindgen!("calculator" in "../rquickjs-wasm-lib/wit/calculator.wit");

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

    let wasi = WasiCtx::builder().inherit_stdio().inherit_args().build();
    let state = ComponentRunStates {
        wasi_ctx: wasi,
        resource_table: ResourceTable::new(),
    };
    let mut store = Store::new(&engine, state);
    // Instantiate it as a normal component
    let component = Component::from_file(&engine, "../rquickjs-wasm-lib/target/wasm32-wasip2/release/rquickjs_wasm_lib.wasm").unwrap();
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
pub unsafe extern "C" fn run(
    ctx: *mut RuntimeContext,
    a: u32, b: u32) -> u32 {
    unsafe {
        let mut ctx = Box::from_raw(ctx as *mut InternalRuntimeContext);
        // let result = ctx.calculator.call_add(&mut ctx.store, a, b).unwrap();
        // result
        let api = ctx.calculator.local_calculator_api();
        let result = api.engine().call_add(&mut ctx.store, ctx.instance, a, b).unwrap();
        let _ = Box::into_raw(ctx);
        result
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn csharp_to_rust(
    ctx: *mut RuntimeContext,
    cb: extern "C" fn(x: i32, y: i32) -> i32,
) {
    let sum = cb(10, 20); // invoke C# method
    println!("{sum}");
}
