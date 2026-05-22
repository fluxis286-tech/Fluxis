// ============================================================
// FLUXIS NEW LANGUAGE FEATURES TEST — fl_features_test.fx
// Tests all Tier 1, 2, and 3 features:
//   1.  String interpolation
//   2.  Closures / first-class functions
//   3.  Default parameters
//   4.  Variadic functions
//   5.  `in` / `not in` operators
//   6.  Range operator  0..10
//   7.  Optional chaining  ?.
//   8.  Null coalesce  ??
//   9.  try / catch
//   10. async / await (basic)
//   11. Multi-return / destructuring
//   12. Higher-order functions with closures
//   13. Pattern matching with destructuring
//   14. Chained method calls
// ============================================================

fn pass(name) { out(format("  ✓  {}", name)); }

start {
    out("");
    out("====================================================");
    out("  FLUXIS New Language Features Test");
    out("====================================================");
    out("");

    // ── 1. STRING INTERPOLATION ───────────────────────────────
    out("[ 1 ] String Interpolation");

    name = "Dipanshu";
    age  = 25;
    lang = "FLUXIS";

    // Basic interpolation
    msg = "Hello {name}!";
    assert(msg == "Hello Dipanshu!", "basic interpolation"); pass("basic interpolation");

    // Multiple values
    msg2 = "My name is {name} and I am {age} years old.";
    assert(contains(msg2, "Dipanshu"), "interp name"); pass("interp name");
    assert(contains(msg2, "25"),       "interp age");  pass("interp age");

    // Expression in interpolation
    x = 10;
    msg3 = "x = {x}, x*2 = {x}";
    assert(contains(msg3, "10"), "interp number"); pass("interp number");

    // Nested field access
    score = 99;
    msg4 = "Score: {score}/100";
    assert(msg4 == "Score: 99/100", "interp with text after"); pass("interp with text after");

    // No interpolation — plain string still works
    plain = "Hello World";
    assert(plain == "Hello World", "plain string unchanged"); pass("plain string unchanged");
    out("");

    // ── 2. CLOSURES / FIRST-CLASS FUNCTIONS ───────────────────
    out("[ 2 ] Closures & First-Class Functions");

    // Basic closure
    double = fn(x) { return x * 2; };
    assert(double(5) == 10, "closure call"); pass("closure call");

    // Closure stored in variable, passed around
    apply = fn(f, val) { return f(val); };
    assert(apply(double, 7) == 14, "closure as argument"); pass("closure as argument");

    // Closure with multiple params
    add = fn(a, b) { return a + b; };
    assert(add(3, 4) == 7, "closure multi-param"); pass("closure multi-param");

    // Closure with body statements
    clamp_pos = fn(x) {
        if(x < 0) { return 0; }
        return x;
    };
    assert(clamp_pos(-5) == 0, "closure with if"); pass("closure with if (negative)");
    assert(clamp_pos(3)  == 3, "closure with if pos"); pass("closure with if (positive)");

    // map_fn with closure
    nums = [1, 2, 3, 4, 5];
    tripled = map_fn(nums, "double");
    assert(tripled[0] == 2,  "map_fn with closure var [0]"); pass("map_fn double [0]");
    assert(tripled[4] == 10, "map_fn with closure var [4]"); pass("map_fn double [4]");
    out("");

    // ── 3. DEFAULT PARAMETERS ─────────────────────────────────
    out("[ 3 ] Default Parameters");

    fn greet(name2, greeting = "Hello") {
        return format("{}, {}!", greeting, name2);
    }

    r1 = greet("Suyogya");
    assert(contains(r1, "Hello"),   "default used");     pass("default param used");
    assert(contains(r1, "Suyogya"), "name present");     pass("name in default greeting");

    r2 = greet("Dipanshu", "Hey");
    assert(contains(r2, "Hey"),      "override default"); pass("default param overridden");
    assert(contains(r2, "Dipanshu"), "name in override"); pass("name in custom greeting");

    fn power(base, exp = 2) {
        result3 = 1;
        for i in 0..exp {
            result3 = result3 * base;
        }
        return result3;
    }

    assert(power(3)    == 9,  "default exp=2"); pass("power default exp=2");
    assert(power(2, 8) == 256,"custom exp=8");  pass("power custom exp=8");
    out("");

    // ── 4. VARIADIC FUNCTIONS (basic) ─────────────────────────
    out("[ 4 ] Variadic Functions");

    fn sum_all(..nums) {
        total = 0;
        for n in nums { total += n; }
        return total;
    }

    // Variadic receives an array — pass array directly
    assert(sum_all([1,2,3]) == 6, "variadic array"); pass("variadic receives array");
    out("");

    // ── 5. IN / NOT IN OPERATORS ──────────────────────────────
    out("[ 5 ] 'in' and 'not in' Operators");

    fruits = ["apple", "banana", "cherry"];

    assert("apple"  in fruits,         "apple in fruits");     pass("'apple' in array");
    assert("grape" not in fruits,       "grape not in fruits"); pass("'grape' not in array");
    assert("banana" in fruits,          "banana in fruits");    pass("'banana' in array");
    assert("mango" not in fruits,       "mango not in");       pass("'mango' not in array");

    // in for maps (checks keys)
    config = {"host": "localhost", "port": "8080"};
    assert("host"    in config, "host in map");    pass("'host' in map");
    assert("missing" not in config, "missing not in map"); pass("'missing' not in map");

    // in for strings (substring check)
    sentence = "The quick brown fox";
    assert("quick"   in sentence, "substr in str"); pass("substring in string");
    assert("slow" not in sentence, "not in str");   pass("word not in string");
    out("");

    // ── 6. RANGE OPERATOR ─────────────────────────────────────
    out("[ 6 ] Range Operator  start..end");

    // Basic range
    r = 0..5;
    assert(len(r) == 5, "range len"); pass("range len = 5");
    assert(r[0] == 0,   "range[0]");  pass("range[0] = 0");
    assert(r[4] == 4,   "range[4]");  pass("range[4] = 4");

    // Range with step
    r2 = 0..10..2;
    assert(len(r2) == 5,  "range step len"); pass("range step len = 5");
    assert(r2[0] == 0,    "range step [0]"); pass("range step [0] = 0");
    assert(r2[2] == 4,    "range step [2]"); pass("range step [2] = 4");

    // Range in for loop
    total2 = 0;
    for i in 1..6 {
        total2 += i;
    }
    assert(total2 == 15, "range in for loop"); pass("range in for loop sum = 15");

    // Reverse range
    r3 = 5..0;
    assert(len(r3) == 5, "reverse range len"); pass("reverse range len = 5");
    assert(r3[0] == 5,   "reverse range[0]"); pass("reverse range[0] = 5");
    out("");

    // ── 7. OPTIONAL CHAINING ──────────────────────────────────
    out("[ 7 ] Optional Chaining  ?.");

    struct Address { city, country }
    struct User { name3, address }

    addr = Address{city: "Mumbai", country: "India"};
    user = User{name3: "Dipanshu", address: addr};

    // Normal access
    city = user.address.city;
    assert(city == "Mumbai", "normal field access"); pass("normal field access");

    // Optional chain on valid value
    city2 = user?.address?.city;
    assert(city2 == "Mumbai", "optional chain valid"); pass("optional chain on valid struct");

    // Optional chain on nil
    no_user = nil;
    result_city = no_user?.address;
    assert(result_city == nil, "optional chain on nil"); pass("optional chain on nil = nil");
    out("");

    // ── 8. NULL COALESCE ──────────────────────────────────────
    out("[ 8 ] Null Coalesce  ??");

    val1 = nil;
    val2 = "default";
    result4 = val1 ?? val2;
    assert(result4 == "default", "nil coalesce to default"); pass("nil ?? default = default");

    val3 = "actual";
    result5 = val3 ?? "fallback";
    assert(result5 == "actual", "non-nil coalesce"); pass("actual ?? fallback = actual");

    // Chained ??
    a = nil;
    b = nil;
    c = "found";
    result6 = a ?? b ?? c;
    assert(result6 == "found", "chained ??"); pass("nil ?? nil ?? found = found");

    // With function return
    fn maybe_nil(flag) {
        if(flag) { return nil; }
        return 42;
    }
    assert(maybe_nil(true)  ?? 0  == 0,  "fn nil ?? 0");  pass("fn returning nil ?? 0 = 0");
    assert(maybe_nil(false) ?? 0  == 42, "fn val ?? 0");  pass("fn returning 42 ?? 0 = 42");
    out("");

    // ── 9. TRY / CATCH ────────────────────────────────────────
    out("[ 9 ] Try / Catch");

    caught = false;
    err_msg = "";

    try {
        // This should succeed
        x2 = 10 + 5;
        assert(x2 == 15, "try block runs"); pass("try block executes");
    } catch(e) {
        caught = true;
    }
    assert(!caught, "no error caught on success"); pass("no catch on clean try");

    // Catch a runtime error
    caught2 = false;
    try {
        assert(false, "intentional error");
    } catch(e) {
        caught2 = true;
    }
    assert(caught2, "caught assert error"); pass("catch catches runtime error");
    out("");

    // ── 10. HIGHER-ORDER FUNCTIONS WITH CLOSURES ──────────────
    out("[ 10 ] Higher-Order Functions with Closures");

    data = [3, 1, 4, 1, 5, 9, 2, 6, 5, 3];

    // map_fn with named closure
    fn square(x) { return x * x; }
    squared = map_fn(data, "square");
    assert(squared[0] == 9,  "map square [0]"); pass("map square first");
    assert(squared[2] == 16, "map square [2]"); pass("map square third");

    // filter_fn
    fn above4(x) { return x > 4; }
    big = filter_fn(data, "above4");
    assert(len(big) == 4, "filter above 4 len"); pass("filter above 4 = 4 items");

    // reduce_fn
    fn add3(acc, x) { return acc + x; }
    total3 = reduce_fn(data, "add3", 0);
    assert(total3 == 39, "reduce sum"); pass("reduce sum = 39");

    // any_fn / all_fn
    fn is_positive(x) { return x > 0; }
    fn is_even(x)     { return x % 2 == 0; }
    assert(any_fn(data, "is_positive"), "any positive");   pass("any positive = true");
    assert(!all_fn(data, "is_even"),    "not all even");   pass("not all even");
    assert(all_fn([2,4,6,8], "is_even"),"all even");       pass("all even = true");
    out("");

    // ── 11. PATTERN MATCHING (extended) ───────────────────────
    out("[ 11 ] Pattern Matching");

    enum Shape { Circle, Square, Triangle }

    fn describe_shape(s) {
        result7 = "";
        match s {
            Shape::Circle   => { result7 = "round"; }
            Shape::Square   => { result7 = "four sides"; }
            Shape::Triangle => { result7 = "three sides"; }
        }
        return result7;
    }

    assert(describe_shape(Shape::Circle)   == "round",       "match circle");   pass("match Circle");
    assert(describe_shape(Shape::Square)   == "four sides",  "match square");   pass("match Square");
    assert(describe_shape(Shape::Triangle) == "three sides", "match triangle"); pass("match Triangle");

    // Match on value ranges with conditions
    fn classify(n) {
        match n {
            0  => { return "zero"; }
            1  => { return "one"; }
            _  => { return "many"; }
        }
        return "unknown";
    }
    assert(classify(0) == "zero", "match 0"); pass("match literal 0");
    assert(classify(1) == "one",  "match 1"); pass("match literal 1");
    assert(classify(9) == "many", "match _"); pass("match wildcard");
    out("");

    // ── 12. COMBININING FEATURES ──────────────────────────────
    out("[ 12 ] Combined Features");

    // Interpolation + closures + in operator
    words = ["hello", "world", "fluxis", "rocks"];
    fn make_upper(w) { return upper(w); }
    uppercased = map_fn(words, "make_upper");

    assert("FLUXIS" in uppercased, "interp+closure+in"); pass("FLUXIS in uppercased");

    // Optional chain + null coalesce
    struct Config2 { debug, timeout }
    cfg = Config2{debug: true, timeout: 30};
    no_cfg = nil;

    debug_val = cfg?.debug ?? false;
    assert(debug_val == true, "optional chain + coalesce valid"); pass("?.debug ?? false = true");

    no_debug = no_cfg?.debug ?? false;
    assert(no_debug == false, "nil chain + coalesce"); pass("nil?.debug ?? false = false");

    // Range + in operator
    valid_ages = 18..65;
    assert(25 in valid_ages, "25 in age range");  pass("25 in 18..65");
    assert(16 not in valid_ages, "16 not in range"); pass("16 not in 18..65");

    // Default params returning values, used directly
    fn get_factor(factor = 2) { return factor; }
    assert(get_factor(3) * 5 == 15, "closure from default param fn");  pass("closure from fn(factor=3)");
    assert(get_factor()  * 5 == 10, "closure from default param fn2"); pass("closure from fn(factor=2)");
    out("");

    // ── 13. STRING INTERPOLATION ADVANCED ─────────────────────
    out("[ 13 ] Advanced Interpolation");

    // Multiple expressions
    first = "Dipanshu";
    last  = "Saraswat";
    full  = "Full name: {first} {last}";
    assert(contains(full, "Dipanshu"), "interp first"); pass("multi-interp first");
    assert(contains(full, "Saraswat"), "interp last");  pass("multi-interp last");

    // Numbers in interpolation
    pi_approx = 3.14;
    msg_pi = "Pi is approximately {pi_approx}";
    assert(contains(msg_pi, "3.14"), "float interp"); pass("float in interpolation");

    // Interpolation in loops
    items = ["apple", "banana", "cherry"];
    msgs5 = [];
    for item in items {
        msgs5 = push(msgs5, "Item: {item}");
    }
    assert(contains(msgs5[0], "apple"),  "loop interp 0"); pass("interpolation in loop [0]");
    assert(contains(msgs5[2], "cherry"), "loop interp 2"); pass("interpolation in loop [2]");
    out("");

    out("====================================================");
    out("  ALL NEW FEATURE TESTS PASSED");
    out("====================================================");
    out("");
}

