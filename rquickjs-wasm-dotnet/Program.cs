using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

Console.WriteLine("Hello, World!");
unsafe
{
    var ctx = RQuickJs.Native.NativeMethods.init();

    Console.WriteLine("After init");

    var nameBytes = System.Text.Encoding.UTF8.GetBytes("foobar");
    fixed (byte* nameP = nameBytes)
    {
        RQuickJs.Native.NativeMethods.register(ctx, nameP, &UnitUnit);
    }

    var scriptBytes = System.Text.Encoding.UTF8.GetBytes(
            """
            1 + 1;
            foobar();
            """
            );
    fixed (byte* scriptP = scriptBytes)
    {
        RQuickJs.Native.NativeMethods.eval(ctx, scriptP);
    }
}

[UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
static void UnitUnit() => Console.WriteLine("Hello from C#");
