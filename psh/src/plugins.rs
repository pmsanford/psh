use anyhow::Result;
use plugin_protos::{plugin::Prompt, Message};
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::WasiCtxBuilder;

pub fn get_prompt() -> Result<String> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
    let module = Module::from_file(&engine, "prompt.wasm")?;
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_args()?
        .build();
    let mut store = Store::new(&engine, wasi);
    let instance = linker.instantiate(&mut store, &module)?;

    let get_prompt = instance.get_func(&mut store, "get_prompt").unwrap();

    let prompt = get_prompt.typed::<(), u32, _>(&store)?;

    let memory = instance.get_memory(&mut store, "memory").unwrap();
    let ptr = prompt.call(&mut store, ())?;
    let buf = memory.data(&mut store);
    let buf = &buf[ptr as usize..];
    let prompt = Prompt::decode_length_delimited(buf)?;

    Ok(prompt.segment)
}
