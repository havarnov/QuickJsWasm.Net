use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    csbindgen::Builder::default()
        .input_extern_file("src/lib.rs")
        .csharp_class_name("NativeMethods")
        .csharp_namespace("RQuickJs.Native")
        .csharp_dll_name("libhost_dotnet")
        .csharp_use_function_pointer(true)
        .csharp_generate_const_filter(|_| true)
        .generate_csharp_file("../rquickjs-wasm-dotnet/RQuickJs/RQuickJs.g.cs")?;
    Ok(())
}
