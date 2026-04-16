using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

Console.WriteLine("Hello, World!");
unsafe
{
    var ctx = RQuickJs.Native.NativeMethods.init();

    Console.WriteLine("After init");

    var x = RQuickJs.Native.NativeMethods.run(ctx, 42, 8);
    Console.WriteLine(x);

    var y = RQuickJs.Native.NativeMethods.run(ctx, 42, 8);
    Console.WriteLine(y);

    RQuickJs.Native.NativeMethods.csharp_to_rust(ctx, &Sum);
}

// C# -> Rust, pass static UnmanagedCallersOnly method with `&`
[UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
static int Sum(int x, int y) => x + y;
