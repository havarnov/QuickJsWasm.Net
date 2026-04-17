use std::ffi::{c_char, CString};

use wasmtime::component::{Component, Instance, Linker, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime::component::HasSelf;

use wasmtime::component::ResourceAny;
use wasmtime::component::Resource;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
// use wasmtime_wasi::p2::bindings::sync::exports::wasi::cli::run::Guest;
use crate::exports::rquickjs::wasm::engine_api::Guest;
// use crate::exports::rquickjs::wasm::engine_api::Callbacks;
use crate::rquickjs::wasm::callbacks;

// wasmtime::component::bindgen!("calculator" in "../rquickjs-wasm-lib/wit/calculator.wit");
wasmtime::component::bindgen!({
    world: "rquickjs",
    path: "../rquickjs-wasm-lib/wit/rquickjs.wit",
    with: {
        // Adjust this key to match the exact WIT package/interface/resource path.
        // Pattern is: "package:interface/resource-interface.resource"
        "rquickjs:wasm/callbacks.f-unit-unit": UnitUnit,
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
    rquickjs: Rquickjs,
    instance: ResourceAny,
}

#[unsafe(no_mangle)]
pub extern "C" fn init() -> *mut RuntimeContext {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();

    crate::callbacks::add_to_linker::<ComponentRunStates, HasSelf<_>>(&mut linker, |state: &mut ComponentRunStates| state).unwrap();

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

    let rquickjs = Rquickjs::instantiate(&mut store, &component, &linker).unwrap();

    let api = rquickjs.rquickjs_wasm_engine_api();

    let engine_instance = api.engine().call_constructor(&mut store).unwrap();

    let ctx = Box::new(InternalRuntimeContext {
        _engine: engine,
        store,
        rquickjs,
        instance: engine_instance,
    });

    Box::into_raw(ctx) as *mut RuntimeContext
}

#[unsafe(no_mangle)]
pub extern "C" fn eval(
    ctx: *mut RuntimeContext,
    script:  *const c_char,
) {
    unsafe {
        let script_str = std::ffi::CStr::from_ptr(script).to_string_lossy();
        let mut ctx = Box::from_raw(ctx as *mut InternalRuntimeContext);
        // let result = ctx.calculator.call_add(&mut ctx.store, a, b).unwrap();
        // result
        let api = ctx.rquickjs.rquickjs_wasm_engine_api();
        api
            .engine()
            .call_eval(&mut ctx.store, ctx.instance, &script_str)
            .unwrap();
        let _ = Box::into_raw(ctx);
    }
}

pub struct UnitUnit {
    func: Box<dyn Fn() -> () + Send + 'static>,
}

impl crate::callbacks::Host for ComponentRunStates {}

impl crate::callbacks::HostFUnitUnit for ComponentRunStates {
    fn call(&mut self, cb: Resource<UnitUnit>) -> wasmtime::Result<()> {
        let _unit_unit: &UnitUnit = self.resource_table.get(&cb)?;
        (_unit_unit.func)();
        Ok(())
    }

    fn drop(&mut self, cb: Resource<UnitUnit>) -> wasmtime::Result<()> {
        let _unit_unit: UnitUnit = self.resource_table.delete(cb)?;
        Ok(())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn register(
    ctx: *mut RuntimeContext,
    name:  *const c_char,
    func: extern "C" fn(),
) {
    unsafe {
        let name_str = std::ffi::CStr::from_ptr(name).to_string_lossy();
        let mut ctx = Box::from_raw(ctx as *mut InternalRuntimeContext);

        let api = ctx.rquickjs.rquickjs_wasm_engine_api();
        let unit_unit = UnitUnit { func: Box::new(move || func()), };
        let res: Resource<callbacks::FUnitUnit> = ctx.store.data_mut().resource_table.push(unit_unit).unwrap();
        let _result = api
            .engine()
            .call_register(&mut ctx.store, ctx.instance, &name_str, res)
            .unwrap();

        let _ = Box::into_raw(ctx);
    }
}


