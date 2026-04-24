use std::ffi::{c_char, CString};
use std::sync::Arc;

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

pub struct Callback {
    func: Arc<dyn Fn(&mut ComponentRunStates, String, Vec<callback_api::Param>) -> callback_api::Param + Send + Sync + 'static>,
}

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
        Ok(match &param.value {
            callback_api::Param::Unit => callback_api::Param::Unit,
            callback_api::Param::Int(i) => callback_api::Param::Int(i.to_owned()),
            callback_api::Param::Str(s) => callback_api::Param::Str(s.to_owned()),
            callback_api::Param::Null => callback_api::Param::Null,
            callback_api::Param::Vec(Some(_)) => todo!("not sure how to do this?"),
            callback_api::Param::Vec(None) => callback_api::Param::Vec(None),
        })
    }

    fn drop(&mut self, resource: Resource<LazyParam>) -> wasmtime::Result<()> {
        let _: LazyParam = self.resource_table.delete(resource)?;
        Ok(())
    }
}

impl crate::callback_api::HostCallback for ComponentRunStates {
    fn invoke(&mut self, resource: Resource<Callback>, name: String, params: Vec<callback_api::Param>) -> Result<callback_api::Param, wasmtime::Error> {
        let func = {
            let callback = self.resource_table.get(&resource)?;
            Arc::clone(&callback.func)
        };

        Ok(func(self, name, params))

    }

    fn drop(&mut self, resource: Resource<Callback>) -> wasmtime::Result<()> {
        let _: Callback = self.resource_table.delete(resource)?;
        Ok(())
    }
}

#[repr(C)]
pub enum ParamTag {
    Unit = 1,
    Int= 2,
    String = 3,
    Null = 4,
    List = 5,
}

#[repr(C)]
pub struct ParamList {
    start: *mut Param,
    len: usize,
}

impl ParamList {
    fn new_null() -> ParamList {
        ParamList {
            start: std::ptr::null_mut(),
            len: 0,
        }
    }
}

#[repr(C)]
pub struct Param {
    tag: ParamTag,
    int_value: i32,
    string_value: *const c_char,
    list_value: ParamList,
}

#[unsafe(no_mangle)]
pub extern "C" fn register(
    ctx: *mut RuntimeContext,
    name: *const c_char,
    func: extern "C" fn(name_ptr: *const c_char, array_ptr: *const Param, array_len: usize) -> *const Param,
) {
    unsafe {
        let name_str = std::ffi::CStr::from_ptr(name).to_string_lossy();
        let mut ctx = Box::from_raw(ctx as *mut InternalRuntimeContext);

        let api = ctx.rquickjs.rquickjs_wasm_engine_api();
        let callback = Callback {
            func: Arc::new(move |component: _, name: String, params: Vec<callback_api::Param>| {
                let name_ptr = CString::new(name)
                    .expect("Couldn't create CString")
                    .into_raw();

                let params: Vec<Param> = params.into_iter()
                    .map(|p| match p {
                        callback_api::Param::Unit => Param { tag: ParamTag::Unit, int_value: 0, string_value: std::ptr::null_mut(), list_value: ParamList::new_null(), },
                        callback_api::Param::Int(value) => Param { tag: ParamTag::Int, int_value: value.unwrap_or(0), string_value: std::ptr::null_mut(), list_value: ParamList::new_null(), },
                        callback_api::Param::Str(None) => Param { tag: ParamTag::String, int_value: 0, string_value: std::ptr::null_mut(), list_value: ParamList::new_null(), },
                        callback_api::Param::Str(Some(value)) => {
                            let string_ptr = CString::new(value)
                                .expect("Couldn't create CString")
                                .into_raw();
                            Param { tag: ParamTag::String, int_value: 0, string_value: string_ptr, list_value: ParamList::new_null(), }
                        },
                        callback_api::Param::Null => Param { tag: ParamTag::Null, int_value: 0, string_value: std::ptr::null_mut(), list_value: ParamList::new_null(), },
                        callback_api::Param::Vec(None) => Param { tag: ParamTag::List, int_value: 0, string_value: std::ptr::null_mut(), list_value: ParamList::new_null(), },
                        callback_api::Param::Vec(Some(value)) => {
                            let mut result = vec![];
                            for inner in value {
                                let inner = crate::callback_api::HostLazyParam::get(component, inner).unwrap();
                                result.push(match inner {
                                    callback_api::Param::Unit => Param { tag: ParamTag::Unit, int_value: 0, string_value: std::ptr::null_mut(), list_value: ParamList::new_null(), },
                                    _ => todo!("add rest"),
                                });
                            }
                            let len = result.len();
                            let data = Box::new(result);
                            let list = ParamList {
                                start: Box::into_raw(data) as *mut Param,
                                len,
                            };
                            Param { tag: ParamTag::List, int_value: 0, string_value: std::ptr::null_mut(), list_value: list, }
                        },
                    })
                    .collect();
                let ptr = params.as_ptr();
                let len = params.len();
                let result: *const Param = func(name_ptr, ptr, len);
                let result = Box::from_raw(result as *mut Param);
                println!("result from host: {:?}", result.int_value);
                match result.tag {
                    ParamTag::Unit => callback_api::Param::Unit,
                    ParamTag::Int => callback_api::Param::Int(Some(result.int_value)),
                    ParamTag::String => {
                        todo!()
                        // callback_api::Param::Str(result.string_value)
                    },
                    ParamTag::Null => callback_api::Param::Null,
                    ParamTag::List => callback_api::Param::Vec(vec![].into()),
                }
            }),
        };
        let res: Resource<callback_api::Callback> =
            ctx.store.data_mut().resource_table.push(callback).unwrap();
        let _result = api
            .engine()
            .call_register(&mut ctx.store, ctx.instance, &name_str, res)
            .unwrap();

        let _ = Box::into_raw(ctx);
    }
}

