use std::ffi::c_char;

use wasmtime::component::HasSelf;
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Engine, Store};

use crate::rquickjs::wasm::callback_api;
use wasmtime::component::Resource;
use wasmtime::component::ResourceAny;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

wasmtime::component::bindgen!({
    world: "rquickjs",
    path: "../rquickjs-wasm-lib/wit/rquickjs.wit",
    with: {
        "rquickjs:wasm/callback-api.callback": Callback,
        "rquickjs:wasm/callback-api.lazy-param": LazyParam,
    },
    imports: { default: trappable },
});

pub struct ComponentRunStates {
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
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
    rquickjs: Rquickjs,
    instance: ResourceAny,
}

#[unsafe(no_mangle)]
pub extern "C" fn init() -> *mut RuntimeContext {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();

    crate::callback_api::add_to_linker::<ComponentRunStates, HasSelf<_>>(
        &mut linker,
        |state: &mut ComponentRunStates| state,
    )
    .unwrap();

    let wasi = WasiCtx::builder().inherit_stdio().inherit_args().build();
    let state = ComponentRunStates {
        wasi_ctx: wasi,
        resource_table: ResourceTable::new(),
    };
    let mut store = Store::new(&engine, state);

    let component = Component::from_file(
        &engine,
        "./rquickjs_wasm_lib.wasm",
    )
    .unwrap();

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
pub extern "C" fn eval(ctx: *mut RuntimeContext, script: *const c_char) {
    unsafe {
        let script_str = std::ffi::CStr::from_ptr(script).to_string_lossy();
        let mut ctx = Box::from_raw(ctx as *mut InternalRuntimeContext);

        let api = ctx.rquickjs.rquickjs_wasm_engine_api();

        api.engine()
            .call_eval(&mut ctx.store, ctx.instance, &script_str)
            .unwrap();

        let _ = Box::into_raw(ctx);
    }
}

pub struct Callback {}

pub struct LazyParam {
    value: callback_api::Param,
}

impl crate::callback_api::Host for ComponentRunStates {}

impl crate::callback_api::HostLazyParam for ComponentRunStates {
    fn new(&mut self, value: callback_api::Param) -> Result<Resource<LazyParam>, wasmtime::Error> {
        let resource: LazyParam = LazyParam { value, };
        let resource: Resource<callback_api::LazyParam> = self.resource_table.push(resource).unwrap();
        Ok(resource)
    }

    fn get(&mut self, resource: wasmtime::component::Resource<LazyParam>) -> Result<callback_api::Param, wasmtime::Error> {
        let param: &LazyParam = self.resource_table.get(&resource)?;
        Ok(match param.value {
            callback_api::Param::Unit => callback_api::Param::Unit,
            callback_api::Param::Int(i) => callback_api::Param::Int(i),
            _ => todo!(),
        })
    }

    fn drop(&mut self, resource: Resource<LazyParam>) -> wasmtime::Result<()> {
        let _: LazyParam = self.resource_table.delete(resource)?;
        Ok(())
    }
}

impl crate::callback_api::HostCallback for ComponentRunStates {
    fn invoke(&mut self, resource: Resource<Callback>, _params: Vec<callback_api::Param>) -> Result<callback_api::Param, wasmtime::Error> {
        let _callback: &Callback = self.resource_table.get(&resource)?;
        // TODO: let result = (callback.invoke)();
        Ok(callback_api::Param::Int(42))
    }

    fn drop(&mut self, resource: Resource<Callback>) -> wasmtime::Result<()> {
        let _: Callback = self.resource_table.delete(resource)?;
        Ok(())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn register(ctx: *mut RuntimeContext, name: *const c_char, _func: extern "C" fn()) {
    unsafe {
        let name_str = std::ffi::CStr::from_ptr(name).to_string_lossy();
        let mut ctx = Box::from_raw(ctx as *mut InternalRuntimeContext);

        let api = ctx.rquickjs.rquickjs_wasm_engine_api();
        let unit_unit = Callback {};
        let res: Resource<callback_api::Callback> =
            ctx.store.data_mut().resource_table.push(unit_unit).unwrap();
        let _result = api
            .engine()
            .call_register(&mut ctx.store, ctx.instance, &name_str, res)
            .unwrap();

        let _ = Box::into_raw(ctx);
    }
}
