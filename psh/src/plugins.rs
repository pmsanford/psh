use std::{borrow::BorrowMut, sync::Arc};

use anyhow::Result;
use plugin_protos::{plugin::Prompt, Message};
use wasmtime::{Engine, Instance, Linker, Module, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

pub struct Plugin {
    store: Store<WasiCtx>,
    instance: Instance,
}

impl Plugin {
    pub fn call_prompt(&mut self) -> Result<String> {
        let func = self
            .instance
            .get_typed_func::<(), u32, _>(&mut self.store, "get_prompt")?;
        let memory = self.instance.get_memory(&mut self.store, "memory").unwrap();
        let ptr = func.call(&mut self.store, ())?;

        let buf = memory.data(&mut self.store);
        let buf = &buf[ptr as usize..];
        let prompt = Prompt::decode_length_delimited(buf)?;

        Ok(prompt.segment)
    }
}

pub fn get_prompt_plugin() -> Result<Plugin> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
    let module = Module::from_file(&engine, "prompt.wasm")?;
    let wasi = WasiCtxBuilder::new()
        .inherit_env()?
        .inherit_stdio()
        .inherit_args()?
        .build();
    let mut store = Store::new(&engine, wasi);
    let instance = linker.instantiate(&mut store, &module)?;

    Ok(Plugin { store, instance })
}
