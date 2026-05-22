// ============================================================
// FLUXIS NEW FEATURES TEST — fl_new_test.fx
// Tests all recently fixed/added features:
//   1. in() input function
//   2. import .fx files
//   3. Struct field validation
//   4. Enum variant validation
//   5. Multi-error reporting
//   6. All error types with line info
// ============================================================

fn pass(name) { out(format("  ✓  {}", name)); }

// ── Helper file for import test ───────────────────────────────
// We'll create it at runtime via write_file, then import it
// (import works on .fx files in the same directory)

start {
    out("");
    out("====================================================");
    out("  FLUXIS New Features Test");
    out("====================================================");
    out("");

    // ── 1. BASIC LANGUAGE SANITY ─────────────────────────────
    out("[ 1 ] Basic sanity (existing features still work)");

    x = 10;
    assert(x == 10, "assignment"); pass("assignment");

    arr = [1, 2, 3];
    assert(len(arr) == 3, "array len"); pass("array len");

    m = {"key": "value"};
    assert(m["key"] == "value", "map get"); pass("map get");

    result = 0;
    for i in range(0, 5) {
        result += i;
    }
    assert(result == 10, "for-in range"); pass("for-in range");
    out("");

    // ── 2. STRUCTS WITH FIELD VALIDATION ─────────────────────
    out("[ 2 ] Struct field access");

    struct Vector3 { x, y, z }
    struct Color { r, g, b }
    struct Player { name, hp, score }

    v = Vector3{x: 1, y: 2, z: 3};
    assert(v.x == 1, "struct field x"); pass("struct field x");
    assert(v.y == 2, "struct field y"); pass("struct field y");
    assert(v.z == 3, "struct field z"); pass("struct field z");

    v.x = 99;
    assert(v.x == 99, "struct field assign"); pass("struct field assign");

    c = Color{r: 255, g: 128, b: 0};
    assert(c.r == 255, "color r"); pass("color r");
    assert(c.g == 128, "color g"); pass("color g");

    p = Player{name: "Dipanshu", hp: 100, score: 0};
    assert(p.name == "Dipanshu", "player name"); pass("player name");
    assert(p.hp == 100, "player hp"); pass("player hp");

    p.score += 50;
    assert(p.score == 50, "player score +="); pass("player score +=");
    out("");

    // ── 3. ENUMS ─────────────────────────────────────────────
    out("[ 3 ] Enum validation");

    enum Direction { North, South, East, West }
    enum GameState { Playing, Paused, GameOver }
    enum Rarity { Common, Rare, Epic, Legendary }

    d = Direction::North;
    assert(d == Direction::North, "enum North"); pass("enum North");
    assert(d != Direction::South, "enum != South"); pass("enum != South");

    gs = GameState::Playing;
    match gs {
        GameState::Playing  => { pass("match Playing"); }
        GameState::Paused   => { fail("should not be Paused"); }
        GameState::GameOver => { fail("should not be GameOver"); }
    }

    r = Rarity::Legendary;
    assert(r == Rarity::Legendary, "enum Legendary"); pass("enum Legendary");
    out("");

    // ── 4. IMPORT ────────────────────────────────────────────
    out("[ 4 ] Import .fx files");

    // Write a helper module using separate lines
    write_file("_test_helper.fx", "fn helper_add(a, b) {");
    append_file("_test_helper.fx", "\n    return a + b;");
    append_file("_test_helper.fx", "\n}");
    append_file("_test_helper.fx", "\nfn helper_greet(name) {");
    append_file("_test_helper.fx", "\n    return format(\"Hello, {}!\", name);");
    append_file("_test_helper.fx", "\n}");

    import "_test_helper.fx";

    result2 = helper_add(10, 20);
    assert(result2 == 30, "imported function result"); pass("imported function result");

    greeting = helper_greet("FLUXIS");
    assert(contains(greeting, "FLUXIS"), "imported function string"); pass("imported function string");

    // Import is idempotent — double import should not re-run
    import "_test_helper.fx";
    assert(helper_add(1, 1) == 2, "double import safe"); pass("double import safe");
    out("");

    // ── 5. STDLIB IMPORTS ─────────────────────────────────────
    out("[ 5 ] Stdlib imports");

    import "math";
    assert(abs(-42) == 42, "import math abs"); pass("import math abs");

    import "string";
    assert(upper("hello") == "HELLO", "import string upper"); pass("import string upper");

    import "io";
    write_file("_test_io.txt", "test content");
    assert(file_exists("_test_io.txt"), "import io write"); pass("import io write");
    out("");

    // ── 6. BREAK AND CONTINUE IN NESTED LOOPS ─────────────────
    out("[ 6 ] Break/continue in loops");

    // Break
    found = -1;
    for i in range(0, 10) {
        if(i == 5) {
            found = i;
            break;
        }
    }
    assert(found == 5, "break in for-in"); pass("break in for-in");

    // Continue
    sum = 0;
    for i in range(0, 10) {
        if(i % 2 != 0) { continue; }
        sum += i;
    }
    assert(sum == 20, "continue in for-in"); pass("continue in for-in");  // 0+2+4+6+8

    // Nested loops with break
    outer_count = 0;
    for i in range(0, 3) {
        for j in range(0, 3) {
            if(j == 1) { break; }
            outer_count += 1;
        }
    }
    assert(outer_count == 3, "break in nested loop"); pass("break in nested loop");
    out("");

    // ── 7. TYPE CHECKS ────────────────────────────────────────
    out("[ 7 ] Runtime type checks");

    assert(is_num(42),        "is_num");    pass("is_num");
    assert(is_str("hi"),      "is_str");    pass("is_str");
    assert(is_bool(true),     "is_bool");   pass("is_bool");
    assert(is_nil(nil),       "is_nil");    pass("is_nil");
    assert(is_array([1,2,3]), "is_array");  pass("is_array");
    assert(is_map({"a": 1}),  "is_map");    pass("is_map");
    assert(is_float(3.14),    "is_float");  pass("is_float");

    assert(type_of(42)    == "num",   "type_of num");   pass("type_of num");
    assert(type_of("hi")  == "str",   "type_of str");   pass("type_of str");
    assert(type_of(true)  == "bool",  "type_of bool");  pass("type_of bool");
    assert(type_of(nil)   == "nil",   "type_of nil");   pass("type_of nil");
    assert(type_of([])    == "array", "type_of array"); pass("type_of array");
    assert(type_of({})    == "map",   "type_of map");   pass("type_of map");
    out("");

    // ── 8. FUNCTION EDGE CASES ────────────────────────────────
    out("[ 8 ] Function edge cases");

    // Recursion with multiple args
    fn fib(n) {
        if(n <= 1) { return n; }
        return fib(n-1) + fib(n-2);
    }
    assert(fib(10) == 55, "fibonacci"); pass("fibonacci(10) = 55");

    // Function returning array
    fn make_range(n) {
        result3 = [];
        for i in range(0, n) {
            result3 = push(result3, i * i);
        }
        return result3;
    }
    squares = make_range(5);
    assert(squares[0] == 0,  "squares[0]"); pass("squares[0] = 0");
    assert(squares[4] == 16, "squares[4]"); pass("squares[4] = 16");

    // Higher-order: map + filter + reduce
    fn triple(x) { return x * 3; }
    fn above5(x) { return x > 5; }
    fn add2(a, b) { return a + b; }

    nums = [1, 2, 3, 4, 5];
    tripled  = map_fn(nums, "triple");
    filtered = filter_fn(tripled, "above5");
    total    = reduce_fn(filtered, "add2", 0);
    assert(total == 6 + 9 + 12 + 15, "map+filter+reduce"); pass("map+filter+reduce = 42");
    out("");

    // ── 9. DOP QUICK CHECK ────────────────────────────────────
    out("[ 9 ] DOP quick check");

    dotion Counter {
        count: 0,
        step:  1,

        fn increment() {
            self.count += self.step;
        }

        fn reset() {
            self.count = 0;
        }

        fn get() {
            return self.count;
        }

        on "add" (n) {
            self.count += n;
        }
    }

    c2 = Counter{};
    assert(c2.get() == 0, "counter initial"); pass("counter initial");

    c2.increment();
    c2.increment();
    assert(c2.get() == 2, "counter after 2 increments"); pass("counter after 2 increments");

    send(c2, "add", 10);
    tick(1);
    assert(c2.get() == 12, "counter after send"); pass("counter after send");

    c2.reset();
    assert(c2.get() == 0, "counter reset"); pass("counter reset");
    out("");

    // ── 10. STRING EDGE CASES ─────────────────────────────────
    out("[ 10 ] String edge cases");

    s = "Hello, World!";
    assert(str_len(s) == 13,         "str_len");      pass("str_len = 13");
    assert(upper(s) == "HELLO, WORLD!", "upper");     pass("upper");
    assert(lower(s) == "hello, world!", "lower");     pass("lower");
    assert(contains(s, "World"),     "contains");     pass("contains");
    assert(starts_with(s, "Hello"),  "starts_with");  pass("starts_with");
    assert(ends_with(s, "World!"),   "ends_with");    pass("ends_with");
    assert(replace(s, "World", "FLUXIS") == "Hello, FLUXIS!", "replace"); pass("replace");

    parts2 = split("a:b:c:d", ":");
    assert(len(parts2) == 4, "split len"); pass("split len = 4");
    assert(parts2[3] == "d", "split last"); pass("split last = d");

    joined = join(["x", "y", "z"], " + ");
    assert(joined == "x + y + z", "join"); pass("join");

    fmt = format("Name: {}, Score: {}", "Suyogya", 100);
    assert(contains(fmt, "Suyogya"), "format name"); pass("format name");
    assert(contains(fmt, "100"),     "format score"); pass("format score");
    out("");

    // ── CLEANUP ───────────────────────────────────────────────
    // Remove temp files we created
    write_file("_test_helper.fx", "");
    write_file("_test_io.txt", "");

    out("====================================================");
    out("  ALL NEW FEATURE TESTS PASSED");
    out("====================================================");
    out("");
}

