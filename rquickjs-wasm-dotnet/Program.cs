using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

using RQuickJs.Native;

Console.WriteLine("Hello, World!");
unsafe
{
    var ctx = RQuickJs.Native.NativeMethods.init();

    Console.WriteLine("After init");

    var nameBytes = System.Text.Encoding.UTF8.GetBytes("foobar");
    fixed (byte* nameP = nameBytes)
    {
        RQuickJs.Native.NativeMethods.register(ctx, nameP, &Callback);
    }

    var scriptBytes = System.Text.Encoding.UTF8.GetBytes(
            """
            1 + 1;
            foobar(10, 20);
            """
            );
    fixed (byte* scriptP = scriptBytes)
    {
        RQuickJs.Native.NativeMethods.eval(ctx, scriptP);
    }
}

[UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
unsafe static Param* Callback(Param* ptr, nuint len)
{
    Span<Param> paramSpan = new Span<Param>(ptr, (int)len);

    // You can now use foreach or indexers
    foreach (ref readonly var p in paramSpan)
    {
        Console.WriteLine($"Tag: {p.tag}, Value: {p.int_value}");
    }

    Param* result_ptr = (Param*)NativeMemory.Alloc((nuint)sizeof(Param));

    // Initialize the values
    result_ptr->tag = ParamTag.Int;
    result_ptr->int_value = 500;

    return result_ptr;
}
