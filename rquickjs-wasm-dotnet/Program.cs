using System.Collections.Concurrent;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using RQuickJs.Native;

Console.WriteLine("Hello, World!");
unsafe
{
    var ctx = RQuickJs.Native.NativeMethods.init();

    Console.WriteLine("After init");

    {
        var nameBytes = System.Text.Encoding.UTF8.GetBytes("foobar");
        fixed (byte* nameP = nameBytes)
        {
            RQuickJs.Native.NativeMethods.register(ctx, nameP, &Callback);
        }
    }

    {
        var nameBytes = System.Text.Encoding.UTF8.GetBytes("add");
        fixed (byte* nameP = nameBytes)
        {
            RQuickJs.Native.NativeMethods.register(ctx, nameP, &Callback);
        }
    }

    var scriptBytes = System.Text.Encoding.UTF8.GetBytes(
            """
            1 + 1;
            foobar(10, 20);
            add(20, 30);
            add(20, 30, 40);
            """
            );
    fixed (byte* scriptP = scriptBytes)
    {
        RQuickJs.Native.NativeMethods.eval(ctx, scriptP);
    }

    var state = new State();

    var foo = new Foo();
    foo.Reg("yolo", () => Console.WriteLine("Hello, YOLO!"));
    foo.Eval("yolo();");

    foo.Reg("add", (int a, int b) => a + b);
    foo.Eval("add(10, 20);");

    foo.Reg("inc", state.Incremental);
    foo.Reg("state_add", state.Add);

    foo.Eval("inc();");
    foo.Eval("state_add(10);");
    foo.Eval("inc();");

    Console.WriteLine("STATE: " + state.Current);
}

[UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
static unsafe Param* Callback(byte* namePtr, Param* ptr, nuint len)
{
    var name = new string((sbyte*)namePtr);
    Console.WriteLine("invoke: " + name);

    if (name == "add")
    {
        Console.WriteLine("HER");
        Span<Param> paramSpan = new Span<Param>(ptr, (int)len);

        int int_result = 0;
        foreach (ref readonly var p in paramSpan)
        {
            int_result += p.int_value;
        }

        Param* result_ptr = (Param*)NativeMemory.Alloc((nuint)sizeof(Param));

        // Initialize the values
        result_ptr->tag = ParamTag.Int;
        result_ptr->int_value = int_result;

        return result_ptr;
    }
    else
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
}

public class State
{
    private int _state = 0;

    public void Incremental() => _state++;
    public void Add(int v) => _state += v;

    public int Current => _state;
}

internal class Foo
{
    private readonly unsafe RuntimeContext* ctx;

    public Foo()
    {
        unsafe
        {
            ctx = NativeMethods.init();
        }
    }

    private static readonly ConcurrentDictionary<string, Delegate>_registry = new();

    public void Eval(string code)
    {
        unsafe
        {
            var scriptBytes = System.Text.Encoding.UTF8.GetBytes(code);

            fixed (byte* scriptP = scriptBytes)
            {
                NativeMethods.eval(ctx, scriptP);
            }
        }
    }

    public void Reg(string name, Delegate func)
    {
        _registry.TryAdd(name, func);

        unsafe
        {
            {
                var nameBytes = System.Text.Encoding.UTF8.GetBytes(name);
                fixed (byte* nameP = nameBytes)
                {
                    NativeMethods.register(ctx, nameP, &InnerCallback);
                }
            }
        }
    }

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static unsafe Param* InnerCallback(byte* namePtr, Param* ptr, nuint len)
    {
        var name = new string((sbyte*)namePtr);

        if (!_registry.TryGetValue(name, out var func))
        {
            throw new NotImplementedException();
        }

        Span<Param> paramSpan = new Span<Param>(ptr, (int)len);


        var idx = 0;
        object?[] parameters = new object?[paramSpan.Length];
        foreach (ref readonly var p in paramSpan)
        {
            parameters[idx] = p.tag switch {
                ParamTag.Unit => throw new NotImplementedException(),
                ParamTag.Int => parameters[idx] = p.int_value,
                // _ => throw new ArgumentOutOfRangeException(),
                _ => parameters[idx] = null,
            };
            idx++;
        }

        var result = func.DynamicInvoke(parameters);

        var isVoid = func.Method.ReturnType == typeof(void);

        Param* result_ptr = (Param*)NativeMemory.Alloc((nuint)sizeof(Param));

        switch (result)
        {
            case null when isVoid:
                result_ptr->tag = ParamTag.Unit;
                break;
            case null:
                break;
            case int value:
                result_ptr->tag = ParamTag.Int;
                result_ptr->int_value = value;
                break;
            default:
                throw new NotImplementedException();
        }

        return result_ptr;
    }
}

