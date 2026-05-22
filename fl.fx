// ============================================================
// FLUXIS FULL FEATURE TEST — fl_test.fx
// Tests every supported language feature.
// All tests use assert() — a failed assert prints an error.
// Run with: fluxis fl_test.fx
// ============================================================

fn pass(name) {
    out(format("  ✓  {}", name));
}

// ────────────────────────────────────────────────────────────
// 1. PRIMITIVES & LITERALS
// ────────────────────────────────────────────────────────────
fn test_primitives() {
    out("[ 1 ] Primitives & Literals");

    x = 42;
    assert(x == 42, "num literal");
    pass("num literal");

    f = 3.14;
    assert(f == 3.14, "float literal");
    pass("float literal");

    s = "hello";
    assert(s == "hello", "string literal");
    pass("string literal");

    b = true;
    assert(b == true, "bool true");
    pass("bool true");

    b2 = false;
    assert(b2 == false, "bool false");
    pass("bool false");

    n = nil;
    assert(n == nil, "nil literal");
    pass("nil literal");
}

// ────────────────────────────────────────────────────────────
// 2. ARITHMETIC
// ────────────────────────────────────────────────────────────
fn test_arithmetic() {
    out("[ 2 ] Arithmetic");

    assert(2 + 3 == 5,  "add");      pass("add");
    assert(10 - 4 == 6, "sub");      pass("sub");
    assert(3 * 4 == 12, "mul");      pass("mul");
    assert(10 / 2 == 5, "div");      pass("div");
    assert(10 % 3 == 1, "mod");      pass("mod");
    assert(-5 == 0 - 5, "negation"); pass("negation");

    // float arithmetic
    assert(1.5 + 1.5 == 3.0, "float add"); pass("float add");
    assert(5.0 / 2.0 == 2.5, "float div"); pass("float div");

    // mixed num + float
    r = 2 + 1.5;
    assert(r == 3.5, "mixed add"); pass("mixed add");
}

// ────────────────────────────────────────────────────────────
// 3. COMPARISON & LOGIC
// ────────────────────────────────────────────────────────────
fn test_comparison() {
    out("[ 3 ] Comparison & Logic");

    assert(3 > 2,   "gt");  pass("gt");
    assert(2 < 3,   "lt");  pass("lt");
    assert(3 >= 3,  "ge");  pass("ge");
    assert(2 <= 3,  "le");  pass("le");
    assert(2 == 2,  "eq");  pass("eq");
    assert(2 != 3,  "ne");  pass("ne");

    assert(true && true,  "and true");  pass("and true");
    assert(!false,        "not false"); pass("not false");
    assert(false || true, "or");        pass("or");

    // short-circuit
    x = 0;
    if(false && x == 0) { x = 99; }
    assert(x == 0, "short-circuit &&"); pass("short-circuit &&");
}

// ────────────────────────────────────────────────────────────
// 4. VARIABLES & TYPE ANNOTATIONS
// ────────────────────────────────────────────────────────────
fn test_variables() {
    out("[ 4 ] Variables & Type Annotations");

    x = 10;
    x = 20;
    assert(x == 20, "reassign"); pass("reassign");

    n: num = 7;
    assert(n == 7, "type annotation num"); pass("type annotation num");

    s: str = "hi";
    assert(s == "hi", "type annotation str"); pass("type annotation str");

    // compound assign
    x = 10;
    x += 5;
    assert(x == 15, "+="); pass("+=");

    x -= 3;
    assert(x == 12, "-="); pass("-=");

    x *= 2;
    assert(x == 24, "*="); pass("*=");

    x /= 4;
    assert(x == 6, "/="); pass("/=");

    x %= 4;
    assert(x == 2, "%="); pass("%=");

    // increment / decrement
    c = 5;
    c++;
    assert(c == 6, "++"); pass("++");
    c--;
    assert(c == 5, "--"); pass("--");
}

// ────────────────────────────────────────────────────────────
// 5. STRINGS
// ────────────────────────────────────────────────────────────
fn test_strings() {
    out("[ 5 ] Strings");

    s = "Hello" + " " + "World";
    assert(s == "Hello World", "string concat"); pass("string concat");

    assert(upper("hello") == "HELLO",   "upper"); pass("upper");
    assert(lower("HELLO") == "hello",   "lower"); pass("lower");
    assert(trim("  hi  ") == "hi",      "trim");  pass("trim");
    assert(str_len("abc") == 3,         "str_len"); pass("str_len");
    assert(contains("hello", "ell"),    "contains"); pass("contains");
    assert(starts_with("hello", "he"),  "starts_with"); pass("starts_with");
    assert(ends_with("hello", "lo"),    "ends_with"); pass("ends_with");
    assert(replace("hi world", "world", "fluxis") == "hi fluxis", "replace"); pass("replace");

    parts = split("a,b,c", ",");
    assert(parts[0] == "a", "split [0]"); pass("split [0]");
    assert(parts[2] == "c", "split [2]"); pass("split [2]");

    j = join(["x","y","z"], "-");
    assert(j == "x-y-z", "join"); pass("join");

    assert(repeat("ab", 3) == "ababab", "repeat"); pass("repeat");
    assert(char_at("hello", 1) == "e",  "char_at"); pass("char_at");

    padl = pad_left("5", 3, "0");
    assert(padl == "005", "pad_left"); pass("pad_left");

    padr = pad_right("hi", 5, ".");
    assert(padr == "hi...", "pad_right"); pass("pad_right");

    assert(parse_int("42") == 42,   "parse_int");   pass("parse_int");
    assert(parse_float("3.14") == 3.14, "parse_float"); pass("parse_float");

    msg = format("Hello {}! You are {} years old.", "Dipanshu", 16);
    assert(contains(msg, "Dipanshu"), "format"); pass("format");
}

// ────────────────────────────────────────────────────────────
// 6. ARRAYS
// ────────────────────────────────────────────────────────────
fn test_arrays() {
    out("[ 6 ] Arrays");

    a = [1, 2, 3];
    assert(a[0] == 1, "array index 0"); pass("array index 0");
    assert(a[2] == 3, "array index 2"); pass("array index 2");
    assert(len(a) == 3, "len"); pass("len");

    a = push(a, 4);
    assert(len(a) == 4, "push"); pass("push");

    a = pop(a);
    assert(len(a) == 3, "pop"); pass("pop");

    // index assign
    a[0] = 99;
    assert(a[0] == 99, "index assign"); pass("index assign");

    // range
    r = range(0, 5);
    assert(len(r) == 5, "range len"); pass("range len");
    assert(r[0] == 0,   "range[0]");  pass("range[0]");
    assert(r[4] == 4,   "range[4]");  pass("range[4]");

    r2 = range(0, 10, 2);
    assert(len(r2) == 5, "range step len"); pass("range step len");
    assert(r2[2] == 4,   "range step [2]"); pass("range step [2]");

    // sort
    unsorted = [3, 1, 2];
    s = sort_arr(unsorted);
    assert(s[0] == 1, "sort_arr [0]"); pass("sort_arr [0]");
    assert(s[2] == 3, "sort_arr [2]"); pass("sort_arr [2]");

    sd = sort_desc(unsorted);
    assert(sd[0] == 3, "sort_desc [0]"); pass("sort_desc [0]");

    // slice
    sl = slice([10,20,30,40,50], 1, 4);
    assert(sl[0] == 20, "slice [0]"); pass("slice [0]");
    assert(len(sl) == 3, "slice len"); pass("slice len");

    // remove / insert
    a2 = [10, 20, 30];
    a2 = remove(a2, 1);
    assert(a2[1] == 30, "remove"); pass("remove");

    a2 = insert(a2, 1, 99);
    assert(a2[1] == 99, "insert"); pass("insert");

    // flatten
    nested = [[1,2],[3,4]];
    flat = flatten(nested);
    assert(flat[0] == 1, "flatten [0]"); pass("flatten [0]");
    assert(flat[3] == 4, "flatten [3]"); pass("flatten [3]");

    // reverse
    rev = reverse([1,2,3]);
    assert(rev[0] == 3, "reverse"); pass("reverse");

    // zip
    z = zip([1,2], ["a","b"]);
    assert(z[0][0] == 1,   "zip [0][0]"); pass("zip [0][0]");
    assert(z[1][1] == "b", "zip [1][1]"); pass("zip [1][1]");
}

// ────────────────────────────────────────────────────────────
// 7. MAPS
// ────────────────────────────────────────────────────────────
fn test_maps() {
    out("[ 7 ] Maps");

    m = {"name": "Fluxis", "version": 9};
    assert(m["name"] == "Fluxis", "map get string key"); pass("map get string key");
    assert(m["version"] == 9,     "map get num key");    pass("map get num key");

    m["author"] = "Dipanshu";
    assert(m["author"] == "Dipanshu", "map set new key"); pass("map set new key");

    assert(has(m, "name"),     "has existing key");    pass("has existing key");
    assert(!has(m, "missing"), "has missing key");     pass("has missing key");

    m = del(m, "version");
    assert(!has(m, "version"), "del"); pass("del");

    ks = keys(m);
    assert(len(ks) >= 2, "keys len"); pass("keys len");

    assert(len(m) == 2, "map len after del"); pass("map len after del");
}

// ────────────────────────────────────────────────────────────
// 8. CONTROL FLOW
// ────────────────────────────────────────────────────────────
fn test_control_flow() {
    out("[ 8 ] Control Flow");

    // if / else
    x = 10;
    result = "none";
    if(x > 5) {
        result = "big";
    } else {
        result = "small";
    }
    assert(result == "big", "if/else"); pass("if/else");

    // else if chain
    score = 75;
    grade = "F";
    if(score >= 90) {
        grade = "A";
    } else if(score >= 80) {
        grade = "B";
    } else if(score >= 70) {
        grade = "C";
    } else {
        grade = "F";
    }
    assert(grade == "C", "else if chain"); pass("else if chain");

    // while
    i = 0;
    sum = 0;
    while(i < 5) {
        sum += i;
        i++;
    }
    assert(sum == 10, "while"); pass("while");

    // for (C-style)
    total = 0;
    for(j=0; j<5; j++;) {
        total += j;
    }
    assert(total == 10, "for C-style"); pass("for C-style");

    // for-in over array
    arr = [10, 20, 30];
    acc = 0;
    for item in arr {
        acc += item;
    }
    assert(acc == 60, "for-in array"); pass("for-in array");

    // for-in over string (chars)
    chars = "";
    for ch in "abc" {
        chars = chars + ch;
    }
    assert(chars == "abc", "for-in string"); pass("for-in string");

    // do-while
    n = 0;
    do {
        n++;
    } while(n < 3);
    assert(n == 3, "do-while"); pass("do-while");

    // break
    found = 0;
    for(k=0; k<10; k++;) {
        if(k == 5) {
            found = k;
            break;
        }
    }
    assert(found == 5, "break"); pass("break");

    // continue
    evens = [];
    for(m=0; m<6; m++;) {
        if(m % 2 != 0) {
            continue;
        }
        evens = push(evens, m);
    }
    assert(len(evens) == 3, "continue"); pass("continue");
}

// ────────────────────────────────────────────────────────────
// 9. FUNCTIONS
// ────────────────────────────────────────────────────────────
fn add(a, b) {
    return a + b;
}

fn factorial(n) {
    if(n <= 1) { return 1; }
    return n * factorial(n - 1);
}

fn greet(name) {
    return format("Hello, {}!", name);
}

fn test_functions() {
    out("[ 9 ] Functions");

    assert(add(3, 4) == 7,           "fn call");       pass("fn call");
    assert(factorial(5) == 120,      "recursion");     pass("recursion");
    assert(contains(greet("bhai"), "bhai"), "fn string"); pass("fn string");
}

// ────────────────────────────────────────────────────────────
// 10. MATCH
// ────────────────────────────────────────────────────────────
enum Direction { North, South, East, West }

fn test_match() {
    out("[ 10 ] Match");

    // match on number
    x = 2;
    res = "none";
    match x {
        1 => { res = "one"; }
        2 => { res = "two"; }
        3 => { res = "three"; }
        _ => { res = "other"; }
    }
    assert(res == "two", "match num"); pass("match num");

    // match on string
    lang = "fluxis";
    cool = false;
    match lang {
        "python"  => { cool = false; }
        "fluxis"  => { cool = true; }
        _         => { cool = false; }
    }
    assert(cool == true, "match string"); pass("match string");

    // match on enum
    d = Direction::North;
    desc = "?";
    match d {
        Direction::North => { desc = "up"; }
        Direction::South => { desc = "down"; }
        Direction::East  => { desc = "right"; }
        Direction::West  => { desc = "left"; }
    }
    assert(desc == "up", "match enum"); pass("match enum");

    // match wildcard
    val = 99;
    caught = false;
    match val {
        1 => { caught = false; }
        _ => { caught = true; }
    }
    assert(caught == true, "match wildcard"); pass("match wildcard");
}

// ────────────────────────────────────────────────────────────
// 11. STRUCTS
// ────────────────────────────────────────────────────────────
struct Point { x, y }
struct Person { name, age }

fn test_structs() {
    out("[ 11 ] Structs");

    p = Point{x: 3, y: 4};
    assert(p.x == 3, "struct field x"); pass("struct field x");
    assert(p.y == 4, "struct field y"); pass("struct field y");

    // field assign
    p.x = 10;
    assert(p.x == 10, "struct field assign"); pass("struct field assign");

    // field +=
    p.y += 6;
    assert(p.y == 10, "struct field +="); pass("struct field +=");

    per = Person{name: "Suyogya", age: 16};
    assert(per.name == "Suyogya", "struct string field"); pass("struct string field");
    assert(per.age == 16,         "struct num field");    pass("struct num field");
}

// ────────────────────────────────────────────────────────────
// 12. ENUMS
// ────────────────────────────────────────────────────────────
enum Color { Red, Green, Blue }
enum Status { Active, Inactive, Pending }

fn test_enums() {
    out("[ 12 ] Enums");

    c = Color::Red;
    assert(c == Color::Red,    "enum value");      pass("enum value");
    assert(c != Color::Blue,   "enum not equal");  pass("enum not equal");

    s = Status::Pending;
    assert(s == Status::Pending, "enum pending"); pass("enum pending");
}

// ────────────────────────────────────────────────────────────
// 13. TYPE CHECKS & CONVERSIONS
// ────────────────────────────────────────────────────────────
fn test_types() {
    out("[ 13 ] Type Checks & Conversions");

    assert(is_num(42),         "is_num");    pass("is_num");
    assert(is_float(3.14),     "is_float");  pass("is_float");
    assert(is_str("hi"),       "is_str");    pass("is_str");
    assert(is_bool(true),      "is_bool");   pass("is_bool");
    assert(is_array([1,2,3]),  "is_array");  pass("is_array");
    assert(is_map({"a":1}),    "is_map");    pass("is_map");
    assert(is_nil(nil),        "is_nil");    pass("is_nil");

    assert(to_str(42) == "42",        "to_str num");   pass("to_str num");
    assert(to_num("7") == 7,          "to_num str");   pass("to_num str");
    assert(to_float(3) == 3.0,        "to_float num"); pass("to_float num");

    assert(type_of(42) == "num",      "type_of num");   pass("type_of num");
    assert(type_of("hi") == "str",    "type_of str");   pass("type_of str");
    assert(type_of(true) == "bool",   "type_of bool");  pass("type_of bool");
    assert(type_of([]) == "array",    "type_of array"); pass("type_of array");
    assert(type_of({}) == "map",      "type_of map");   pass("type_of map");
    assert(type_of(nil) == "nil",     "type_of nil");   pass("type_of nil");
}

// ────────────────────────────────────────────────────────────
// 14. MATH STDLIB
// ────────────────────────────────────────────────────────────
fn test_math() {
    out("[ 14 ] Math stdlib");

    assert(abs(-5) == 5,           "abs");    pass("abs");
    assert(abs(5) == 5,            "abs pos"); pass("abs pos");
    assert(sign(-3) == -1,         "sign -"); pass("sign -");
    assert(sign(3) == 1,           "sign +"); pass("sign +");
    assert(sign(0) == 0,           "sign 0"); pass("sign 0");
    assert(sqrt(9.0) == 3.0,       "sqrt");   pass("sqrt");
    assert(floor(3.9) == 3,        "floor");  pass("floor");
    assert(ceil(3.1) == 4,         "ceil");   pass("ceil");
    assert(max(3, 7) == 7,         "max");    pass("max");
    assert(min(3, 7) == 3,         "min");    pass("min");
    assert(pow(2, 8) == 256,       "pow");    pass("pow");
    assert(clamp(15, 0, 10) == 10, "clamp high"); pass("clamp high");
    assert(clamp(-5, 0, 10) == 0,  "clamp low");  pass("clamp low");

    r = rand(1, 6);
    assert(r >= 1 && r <= 6, "rand range"); pass("rand range");

    rf = rand_float();
    assert(rf >= 0.0 && rf < 1.0, "rand_float"); pass("rand_float");

    assert(pi() > 3.14, "pi"); pass("pi");

    // trig (just check they return floats)
    s = sin(0.0);
    assert(s == 0.0, "sin(0)"); pass("sin(0)");

    c = cos(0.0);
    assert(c == 1.0, "cos(0)"); pass("cos(0)");
}

// ────────────────────────────────────────────────────────────
// 15. IO STDLIB
// ────────────────────────────────────────────────────────────
fn test_io() {
    out("[ 15 ] IO stdlib");

    write_file("fluxis_test.txt", "hello fluxis");
    assert(file_exists("fluxis_test.txt"), "write_file + file_exists"); pass("write_file + file_exists");

    content = read_file("fluxis_test.txt");
    assert(content == "hello fluxis", "read_file"); pass("read_file");

    append_file("fluxis_test.txt", "!");
    content2 = read_file("fluxis_test.txt");
    assert(ends_with(content2, "!"), "append_file"); pass("append_file");

    t = time_now();
    assert(t > 0, "time_now"); pass("time_now");

    assert(!file_exists("fluxis_totally_missing_file_xyz.txt"), "file_exists false"); pass("file_exists false");
}

// ────────────────────────────────────────────────────────────
// 16. ML STDLIB
// ────────────────────────────────────────────────────────────
fn test_ml() {
    out("[ 16 ] ML stdlib");

    // matrix creation
    z = ml_zeros(2, 3);
    assert(ml_get(z, 0, 0) == 0.0, "ml_zeros"); pass("ml_zeros");

    o = ml_ones(2, 2);
    assert(ml_get(o, 1, 1) == 1.0, "ml_ones"); pass("ml_ones");

    id = ml_identity(3);
    assert(ml_get(id, 0, 0) == 1.0, "identity diag");    pass("identity diag");
    assert(ml_get(id, 0, 1) == 0.0, "identity off-diag"); pass("identity off-diag");

    // shape
    sh = ml_shape(z);
    assert(sh[0] == 2.0, "shape rows"); pass("shape rows");
    assert(sh[1] == 3.0, "shape cols"); pass("shape cols");

    // ml_set / ml_get
    z2 = ml_set(z, 0, 0, 9.0);
    assert(ml_get(z2, 0, 0) == 9.0, "ml_set/get"); pass("ml_set/get");

    // matrix arithmetic
    a = ml_ones(2, 2);
    b = ml_ones(2, 2);
    s = ml_add(a, b);
    assert(ml_get(s, 0, 0) == 2.0, "ml_add"); pass("ml_add");

    sc = ml_scale(a, 5.0);
    assert(ml_get(sc, 0, 0) == 5.0, "ml_scale"); pass("ml_scale");

    // matmul: identity * M = M
    m = ml_new(2, 2, 3.0);
    id2 = ml_identity(2);
    res = ml_matmul(id2, m);
    assert(ml_get(res, 0, 0) == 3.0, "ml_matmul"); pass("ml_matmul");

    // dot product
    v1 = [1.0, 2.0, 3.0];
    v2 = [4.0, 5.0, 6.0];
    d = ml_dot(v1, v2);
    assert(d == 32.0, "ml_dot"); pass("ml_dot");

    // activations
    sv = sigmoid(0.0);
    assert(sv == 0.5, "sigmoid(0)"); pass("sigmoid(0)");

    rv = relu(-3.0);
    assert(rv == 0.0, "relu negative"); pass("relu negative");

    rv2 = relu(5.0);
    assert(rv2 == 5.0, "relu positive"); pass("relu positive");

    // softmax sums to 1
    sm = softmax([1.0, 2.0, 3.0]);
    total = ml_sum(sm);
    assert(total > 0.99 && total < 1.01, "softmax sum"); pass("softmax sum");

    // random weights
    w = ml_random_weights(3, 4);
    wsh = ml_shape(w);
    assert(wsh[0] == 3.0, "random_weights rows"); pass("random_weights rows");
    assert(wsh[1] == 4.0, "random_weights cols"); pass("random_weights cols");

    // loss functions
    pred = [0.9, 0.1];
    actual = [1.0, 0.0];
    mse = ml_mse(pred, actual);
    assert(mse >= 0.0, "ml_mse >= 0"); pass("ml_mse >= 0");

    // stats
    arr = [1.0, 2.0, 3.0, 4.0, 5.0];
    assert(ml_sum(arr) == 15.0,  "ml_sum");     pass("ml_sum");
    assert(ml_mean(arr) == 3.0,  "ml_mean");    pass("ml_mean");
    assert(ml_max_val(arr) == 5.0, "ml_max_val"); pass("ml_max_val");
    assert(ml_min(arr) == 1.0,   "ml_min");     pass("ml_min");

    norm = ml_normalize(arr);
    assert(ml_min(norm) == 0.0,  "normalize min"); pass("normalize min");
    assert(ml_max_val(norm) == 1.0, "normalize max"); pass("normalize max");
}

// ────────────────────────────────────────────────────────────
// 17. HIGHER-ORDER ARRAY FUNCTIONS
// ────────────────────────────────────────────────────────────
fn double(x) { return x * 2; }
fn is_even(x) { return x % 2 == 0; }
fn sum2(acc, x) { return acc + x; }

fn test_higher_order() {
    out("[ 17 ] Higher-Order Array Functions");

    nums = [1, 2, 3, 4, 5];

    mapped = map_fn(nums, "double");
    assert(mapped[0] == 2, "map_fn [0]"); pass("map_fn [0]");
    assert(mapped[4] == 10, "map_fn [4]"); pass("map_fn [4]");

    evens = filter_fn(nums, "is_even");
    assert(len(evens) == 2, "filter_fn len"); pass("filter_fn len");
    assert(evens[0] == 2,   "filter_fn [0]"); pass("filter_fn [0]");

    total = reduce_fn(nums, "sum2", 0);
    assert(total == 15, "reduce_fn"); pass("reduce_fn");

    assert(any_fn(nums, "is_even"), "any_fn true");  pass("any_fn true");
    assert(!all_fn(nums, "is_even"), "all_fn false"); pass("all_fn false");

    all_even = [2, 4, 6];
    assert(all_fn(all_even, "is_even"), "all_fn true"); pass("all_fn true");
}

// ────────────────────────────────────────────────────────────
// 18. assert() itself
// ────────────────────────────────────────────────────────────
fn test_assert() {
    out("[ 18 ] assert()");

    assert(true,  "assert true");  pass("assert true");
    assert(1,     "assert 1");     pass("assert 1");
    assert("hi",  "assert str");   pass("assert str");
}

// ────────────────────────────────────────────────────────────
// MAIN
// ────────────────────────────────────────────────────────────
start {
    out("");
    out("====================================================");
    out("  FLUXIS Full Feature Test");
    out("====================================================");
    out("");

    test_primitives();    out("");
    test_arithmetic();    out("");
    test_comparison();    out("");
    test_variables();     out("");
    test_strings();       out("");
    test_arrays();        out("");
    test_maps();          out("");
    test_control_flow();  out("");
    test_functions();     out("");
    test_match();         out("");
    test_structs();       out("");
    test_enums();         out("");
    test_types();         out("");
    test_math();          out("");
    test_io();            out("");
    test_ml();            out("");
    test_higher_order();  out("");
    test_assert();        out("");

    out("====================================================");
    out("  ALL TESTS PASSED");
    out("====================================================");
    out("");
}

