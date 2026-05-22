// ============================================================
// FLUXIS MASTER TEST SUITE — fl_master_test.fx
// Tests EVERY feature of FLUXIS v9.0 in one file.
// Run: fluxis fl_master_test.fx
// ============================================================

fn pass(name) { out(format("  ✓  {}", name)); }
fn fail(name) { out(format("  ✗  FAIL: {}", name)); assert(false, name); }
fn section(n, title) { out(format("\n[ {} ] {}", n, title)); }

// ═══════════════════════════════════════════════════════════
// SECTION 1 — PRIMITIVES
// ═══════════════════════════════════════════════════════════
fn test_primitives() {
    section(1, "Primitives");
    assert(42 == 42,        "int");         pass("integer literal");
    assert(3.14 == 3.14,    "float");       pass("float literal");
    assert("hi" == "hi",    "string");      pass("string literal");
    assert(true == true,    "bool true");   pass("bool true");
    assert(false == false,  "bool false");  pass("bool false");
    assert(nil == nil,      "nil");         pass("nil literal");
}

// ═══════════════════════════════════════════════════════════
// SECTION 2 — ARITHMETIC
// ═══════════════════════════════════════════════════════════
fn test_arithmetic() {
    section(2, "Arithmetic");
    assert(2 + 3   == 5,   "+");   pass("add");
    assert(10 - 4  == 6,   "-");   pass("sub");
    assert(3 * 4   == 12,  "*");   pass("mul");
    assert(10 / 2  == 5,   "/");   pass("div");
    assert(10 % 3  == 1,   "%");   pass("mod");
    assert(-5      == 0-5, "neg"); pass("negation");
    assert(1.5+1.5 == 3.0, "f+");  pass("float add");
    assert(5.0/2.0 == 2.5, "f/");  pass("float div");
    assert(2+1.5   == 3.5, "mix"); pass("mixed add");
    x = 10; x += 5;  assert(x == 15, "+="); pass("+=");
    x -= 3;           assert(x == 12, "-="); pass("-=");
    x *= 2;           assert(x == 24, "*="); pass("*=");
    x /= 4;           assert(x == 6,  "/="); pass("/=");
    x %= 4;           assert(x == 2,  "%="); pass("%=");
    c = 5; c++;       assert(c == 6,  "++"); pass("++");
    c--;              assert(c == 5,  "--"); pass("--");
}

// ═══════════════════════════════════════════════════════════
// SECTION 3 — COMPARISON & LOGIC
// ═══════════════════════════════════════════════════════════
fn test_logic() {
    section(3, "Comparison & Logic");
    assert(3 > 2,   ">");  pass(">");
    assert(2 < 3,   "<");  pass("<");
    assert(3 >= 3,  ">="); pass(">=");
    assert(2 <= 3,  "<="); pass("<=");
    assert(2 == 2,  "=="); pass("==");
    assert(2 != 3,  "!="); pass("!=");
    assert(true && true,  "&&"); pass("&&");
    assert(false || true, "||"); pass("||");
    assert(!false,        "!");  pass("!");
}

// ═══════════════════════════════════════════════════════════
// SECTION 4 — STRINGS
// ═══════════════════════════════════════════════════════════
fn test_strings() {
    section(4, "Strings");
    assert("a"+"b"+"c" == "abc",         "concat");      pass("concat");
    assert(upper("hello") == "HELLO",    "upper");       pass("upper");
    assert(lower("HELLO") == "hello",    "lower");       pass("lower");
    assert(trim("  hi  ") == "hi",       "trim");        pass("trim");
    assert(str_len("abc") == 3,          "str_len");     pass("str_len");
    assert(contains("hello","ell"),      "contains");    pass("contains");
    assert(starts_with("hello","he"),    "starts_with"); pass("starts_with");
    assert(ends_with("hello","lo"),      "ends_with");   pass("ends_with");
    assert(replace("hi world","world","fluxis")=="hi fluxis","replace"); pass("replace");
    p = split("a,b,c", ",");
    assert(p[0]=="a" && p[2]=="c",       "split");       pass("split");
    assert(join(["x","y","z"],"-")=="x-y-z","join");     pass("join");
    assert(repeat("ab",3)=="ababab",     "repeat");      pass("repeat");
    assert(char_at("hello",1)=="e",      "char_at");     pass("char_at");
    assert(parse_int("42")==42,          "parse_int");   pass("parse_int");
    assert(parse_float("3.14")==3.14,    "parse_float"); pass("parse_float");
    assert(pad_left("5",3,"0")=="005",   "pad_left");    pass("pad_left");
    assert(pad_right("hi",4,".")=="hi..","pad_right");   pass("pad_right");
    f = format("Hello {}!", "world");
    assert(contains(f,"world"),          "format");      pass("format");
}

// ═══════════════════════════════════════════════════════════
// SECTION 5 — STRING INTERPOLATION
// ═══════════════════════════════════════════════════════════
fn test_interpolation() {
    section(5, "String Interpolation");
    name = "FLUXIS";
    ver  = 9;
    msg  = "Hello {name} v{ver}!";
    assert(contains(msg,"FLUXIS"),  "interp str"); pass("interpolate string");
    assert(contains(msg,"9"),       "interp num"); pass("interpolate number");
    pi2 = 3.14;
    msg2 = "Pi={pi2}";
    assert(contains(msg2,"3.14"),   "interp float"); pass("interpolate float");
    items = ["a","b","c"];
    msgs = [];
    for item in items {
        msgs = push(msgs, "Item:{item}");
    }
    assert(contains(msgs[0],"a"),   "interp loop"); pass("interpolate in loop");
    assert(contains(msgs[2],"c"),   "interp loop2"); pass("interpolate loop end");
}

// ═══════════════════════════════════════════════════════════
// SECTION 6 — ARRAYS
// ═══════════════════════════════════════════════════════════
fn test_arrays() {
    section(6, "Arrays");
    a = [1,2,3];
    assert(a[0]==1 && a[2]==3,        "index");      pass("index");
    assert(len(a)==3,                 "len");        pass("len");
    a = push(a,4); assert(len(a)==4,  "push");       pass("push");
    a = pop(a);    assert(len(a)==3,  "pop");        pass("pop");
    a[0]=99;       assert(a[0]==99,   "idx assign"); pass("index assign");
    s = sort_arr([3,1,2]);
    assert(s[0]==1 && s[2]==3,        "sort_arr");   pass("sort_arr");
    sd = sort_desc([3,1,2]);
    assert(sd[0]==3,                  "sort_desc");  pass("sort_desc");
    sl = slice([10,20,30,40],1,3);
    assert(sl[0]==20 && len(sl)==2,   "slice");      pass("slice");
    a2 = remove([10,20,30],1);
    assert(a2[1]==30,                 "remove");     pass("remove");
    a3 = insert([10,30],1,20);
    assert(a3[1]==20,                 "insert");     pass("insert");
    fl = flatten([[1,2],[3,4]]);
    assert(fl[3]==4,                  "flatten");    pass("flatten");
    rv = reverse([1,2,3]);
    assert(rv[0]==3,                  "reverse");    pass("reverse");
    z = zip([1,2],["a","b"]);
    assert(z[0][0]==1 && z[1][1]=="b","zip");       pass("zip");
}

// ═══════════════════════════════════════════════════════════
// SECTION 7 — RANGE OPERATOR
// ═══════════════════════════════════════════════════════════
fn test_range() {
    section(7, "Range Operator");
    r = 0..5;
    assert(len(r)==5 && r[0]==0 && r[4]==4, "range basic"); pass("0..5");
    r2 = 0..10..2;
    assert(len(r2)==5 && r2[2]==4,          "range step");  pass("0..10..2");
    r3 = 5..0;
    assert(len(r3)==5 && r3[0]==5,          "range rev");   pass("5..0 reverse");
    total = 0;
    for i in 1..6 { total += i; }
    assert(total==15,                        "range loop");  pass("range in for loop");
}

// ═══════════════════════════════════════════════════════════
// SECTION 8 — MAPS
// ═══════════════════════════════════════════════════════════
fn test_maps() {
    section(8, "Maps");
    m = {"name":"FLUXIS","ver":9};
    assert(m["name"]=="FLUXIS",    "get str");  pass("map get str");
    assert(m["ver"]==9,            "get num");  pass("map get num");
    m["lang"]="DOP";
    assert(m["lang"]=="DOP",       "set");      pass("map set");
    assert(has(m,"name"),          "has yes");  pass("has existing");
    assert(!has(m,"nope"),         "has no");   pass("has missing");
    m=del(m,"ver");
    assert(!has(m,"ver"),          "del");      pass("del");
    ks=keys(m);
    assert(len(ks)>=2,             "keys");     pass("keys");
    assert(len(m)==2,              "map len");  pass("map len");
}

// ═══════════════════════════════════════════════════════════
// SECTION 9 — CONTROL FLOW
// ═══════════════════════════════════════════════════════════
fn test_control() {
    section(9, "Control Flow");
    // if/else
    x = 10; r = "";
    if(x>5){ r="big"; }else{ r="small"; }
    assert(r=="big","if"); pass("if/else");
    // else-if
    s = 75; g = "F";
    if(s>=90){g="A";}else if(s>=80){g="B";}else if(s>=70){g="C";}else{g="F";}
    assert(g=="C","elseif"); pass("else-if chain");
    // while
    i=0; sm=0; while(i<5){sm+=i;i++;} assert(sm==10,"while"); pass("while");
    // C-for
    t=0; for(j=0;j<5;j++;){t+=j;} assert(t==10,"for"); pass("C-style for");
    // for-in array
    acc=0; for item in [10,20,30]{acc+=item;} assert(acc==60,"forin"); pass("for-in array");
    // for-in string
    chars=""; for ch in "abc"{chars=chars+ch;} assert(chars=="abc","forstr"); pass("for-in string");
    // do-while
    n=0; do{n++;}while(n<3); assert(n==3,"dowhile"); pass("do-while");
    // break
    found=0; for(k=0;k<10;k++;){if(k==5){found=k;break;}} assert(found==5,"break"); pass("break");
    // continue
    evens=[]; for(m2=0;m2<6;m2++;){if(m2%2!=0){continue;} evens=push(evens,m2);}
    assert(len(evens)==3,"continue"); pass("continue");
    // nested break
    outer=0; for i2 in 0..3{for j2 in 0..3{if(j2==1){break;} outer+=1;}}
    assert(outer==3,"nested break"); pass("nested break");
}

// ═══════════════════════════════════════════════════════════
// SECTION 10 — FUNCTIONS
// ═══════════════════════════════════════════════════════════
fn add(a, b) { return a + b; }
fn factorial(n) { if(n<=1){return 1;} return n*factorial(n-1); }
fn greet_fn(name) { return format("Hello, {}!", name); }

fn test_functions() {
    section(10, "Functions");
    assert(add(3,4)==7,             "call");   pass("fn call");
    assert(factorial(5)==120,       "recur");  pass("recursion");
    assert(contains(greet_fn("bhai"),"bhai"),"str fn"); pass("fn returns str");
}

// ═══════════════════════════════════════════════════════════
// SECTION 11 — CLOSURES
// ═══════════════════════════════════════════════════════════
fn test_closures() {
    section(11, "Closures");
    double = fn(x){ return x*2; };
    assert(double(5)==10,    "basic closure");   pass("closure call");
    apply = fn(f,val){ return f(val); };
    assert(apply(double,7)==14,"pass closure");  pass("closure as arg");
    add2 = fn(a,b){ return a+b; };
    assert(add2(3,4)==7,     "multi param");     pass("closure multi-param");
    clamp2 = fn(x){ if(x<0){return 0;} return x; };
    assert(clamp2(-3)==0,    "closure if neg");  pass("closure if negative");
    assert(clamp2(5)==5,     "closure if pos");  pass("closure if positive");
}

// ═══════════════════════════════════════════════════════════
// SECTION 12 — DEFAULT PARAMETERS
// ═══════════════════════════════════════════════════════════
fn greet2(name2, greeting = "Hello") {
    return format("{}, {}!", greeting, name2);
}
fn power2(base2, exp2 = 2) {
    r = 1;
    for i in 0..exp2 { r = r * base2; }
    return r;
}

fn test_defaults() {
    section(12, "Default Parameters");
    r1 = greet2("Suyogya");
    assert(contains(r1,"Hello"),   "default used");     pass("default param used");
    assert(contains(r1,"Suyogya"),"name in default");   pass("name with default");
    r2 = greet2("Dev","Hey");
    assert(contains(r2,"Hey"),     "override");         pass("override default");
    assert(power2(3)==9,           "default exp=2");    pass("power default");
    assert(power2(2,8)==256,       "custom exp=8");     pass("power custom");
}

// ═══════════════════════════════════════════════════════════
// SECTION 13 — IN / NOT IN
// ═══════════════════════════════════════════════════════════
fn test_in_operator() {
    section(13, "in / not in");
    fruits = ["apple","banana","cherry"];
    assert("apple" in fruits,       "in arr");   pass("in array");
    assert("grape" not in fruits,   "not in");   pass("not in array");
    m = {"host":"localhost"};
    assert("host" in m,             "in map");   pass("in map");
    assert("port" not in m,         "not map");  pass("not in map");
    s = "The quick brown fox";
    assert("quick" in s,            "in str");   pass("in string");
    assert("slow" not in s,         "not str");  pass("not in string");
}

// ═══════════════════════════════════════════════════════════
// SECTION 14 — OPTIONAL CHAIN & NULL COALESCE
// ═══════════════════════════════════════════════════════════
struct Addr { city, country }
struct Usr  { uname, addr  }

fn test_optional_null() {
    section(14, "Optional Chain & Null Coalesce");
    a = Addr{city:"Mumbai",country:"India"};
    u = Usr{uname:"Dev",addr:a};
    assert(u?.addr?.city=="Mumbai",  "chain valid");   pass("?. valid");
    no = nil;
    assert(no?.addr==nil,            "chain nil");     pass("?. on nil");
    assert(nil ?? "def" == "def",    "coalesce nil");  pass("nil ?? default");
    assert("val" ?? "def" == "val",  "coalesce val");  pass("val ?? fallback");
    assert(nil ?? nil ?? "c" == "c", "chain ??");      pass("chained ??");
}

// ═══════════════════════════════════════════════════════════
// SECTION 15 — TRY / CATCH
// ═══════════════════════════════════════════════════════════
fn test_try_catch() {
    section(15, "Try / Catch");
    caught = false;
    try { x2 = 1 + 1; assert(x2==2,"try ok"); } catch(e) { caught=true; }
    assert(!caught,              "no catch on ok"); pass("try succeeds");
    caught2 = false;
    try { assert(false,"boom"); } catch(e) { caught2=true; }
    assert(caught2,              "caught error");   pass("catch runtime error");
    msg = "";
    try { assert(false,"intentional"); } catch(e) { msg=e; }
    assert(str_len(msg)>0,       "error msg");      pass("catch has error msg");
}

// ═══════════════════════════════════════════════════════════
// SECTION 16 — MATCH
// ═══════════════════════════════════════════════════════════
enum Color2 { Red, Green, Blue }
enum Status { Active, Inactive }

fn test_match() {
    section(16, "Match");
    x = 2; res = "";
    match x { 1=>{res="one";} 2=>{res="two";} _=>{res="other";} }
    assert(res=="two","match num"); pass("match number");
    res = "";
    match "hi" { "hi"=>{res="got hi";} _=>{res="other";} }
    assert(res=="got hi","match str"); pass("match string");
    c = Color2::Green; res = "";
    match c {
        Color2::Red   => { res="red"; }
        Color2::Green => { res="green"; }
        Color2::Blue  => { res="blue"; }
    }
    assert(res=="green","match enum"); pass("match enum");
    res = "";
    match 99 { 1=>{res="one";} _=>{res="wild";} }
    assert(res=="wild","wildcard"); pass("match wildcard");
}

// ═══════════════════════════════════════════════════════════
// SECTION 17 — STRUCTS
// ═══════════════════════════════════════════════════════════
struct Vec2 { x, y }
struct Person { pname, age }

fn test_structs() {
    section(17, "Structs");
    v = Vec2{x:3,y:4};
    assert(v.x==3 && v.y==4,  "init");          pass("struct init");
    v.x=10; assert(v.x==10,   "assign");        pass("field assign");
    v.y+=6; assert(v.y==10,   "compound");      pass("field +=");
    p = Person{pname:"Dev",age:16};
    assert(p.pname=="Dev",    "str field");     pass("struct str field");
    assert(p.age==16,         "num field");     pass("struct num field");
}

// ═══════════════════════════════════════════════════════════
// SECTION 18 — ENUMS
// ═══════════════════════════════════════════════════════════
enum Dir { North, South, East, West }

fn test_enums() {
    section(18, "Enums");
    d = Dir::North;
    assert(d==Dir::North,  "eq");  pass("enum eq");
    assert(d!=Dir::South,  "ne");  pass("enum ne");
    r = ""; match d { Dir::North=>{r="up";} Dir::South=>{r="down";} Dir::East=>{r="right";} Dir::West=>{r="left";} }
    assert(r=="up","match"); pass("enum match");
}

// ═══════════════════════════════════════════════════════════
// SECTION 19 — TYPE CHECKS & CONVERSION
// ═══════════════════════════════════════════════════════════
fn test_types() {
    section(19, "Types");
    assert(is_num(42),          "is_num");    pass("is_num");
    assert(is_float(3.14),      "is_float");  pass("is_float");
    assert(is_str("hi"),        "is_str");    pass("is_str");
    assert(is_bool(true),       "is_bool");   pass("is_bool");
    assert(is_array([1]),       "is_array");  pass("is_array");
    assert(is_map({"a":1}),     "is_map");    pass("is_map");
    assert(is_nil(nil),         "is_nil");    pass("is_nil");
    assert(to_str(42)=="42",    "to_str");    pass("to_str");
    assert(to_num("7")==7,      "to_num");    pass("to_num");
    assert(to_float(3)==3.0,    "to_float");  pass("to_float");
    assert(type_of(42)=="num",  "t_num");     pass("type_of num");
    assert(type_of("hi")=="str","t_str");     pass("type_of str");
    assert(type_of([])==  "array","t_arr");   pass("type_of array");
    assert(type_of(nil)=="nil", "t_nil");     pass("type_of nil");
}

// ═══════════════════════════════════════════════════════════
// SECTION 20 — MATH STDLIB
// ═══════════════════════════════════════════════════════════
fn test_math() {
    section(20, "Math Stdlib");
    assert(abs(-5)==5,           "abs");       pass("abs");
    assert(sign(-3)==-1,         "sign-");     pass("sign neg");
    assert(sign(3)==1,           "sign+");     pass("sign pos");
    assert(sqrt(9.0)==3.0,       "sqrt");      pass("sqrt");
    assert(floor(3.9)==3,        "floor");     pass("floor");
    assert(ceil(3.1)==4,         "ceil");      pass("ceil");
    assert(max(3,7)==7,          "max");       pass("max");
    assert(min(3,7)==3,          "min");       pass("min");
    assert(pow(2,8)==256,        "pow");       pass("pow");
    assert(clamp(15,0,10)==10,   "clamp hi");  pass("clamp high");
    assert(clamp(-5,0,10)==0,    "clamp lo");  pass("clamp low");
    r = rand(1,6); assert(r>=1&&r<=6,"rand");  pass("rand range");
    assert(pi()>3.14,            "pi");        pass("pi");
    assert(sin(0.0)==0.0,        "sin");       pass("sin(0)");
    assert(cos(0.0)==1.0,        "cos");       pass("cos(0)");
}

// ═══════════════════════════════════════════════════════════
// SECTION 21 — IO STDLIB
// ═══════════════════════════════════════════════════════════
fn test_io() {
    section(21, "IO Stdlib");
    write_file("_master_test.txt","hello fluxis");
    assert(file_exists("_master_test.txt"),            "exists");     pass("write+exists");
    assert(read_file("_master_test.txt")=="hello fluxis","read");     pass("read_file");
    append_file("_master_test.txt","!");
    assert(ends_with(read_file("_master_test.txt"),"!"),"append");   pass("append_file");
    assert(!file_exists("_nope_xyz_.txt"),             "not exists"); pass("file_exists false");
    assert(time_now()>0,                               "time");       pass("time_now");
    write_file("_master_test.txt",""); // cleanup
}

// ═══════════════════════════════════════════════════════════
// SECTION 22 — ML STDLIB
// ═══════════════════════════════════════════════════════════
fn test_ml() {
    section(22, "ML Stdlib");
    z = ml_zeros(2,3); assert(ml_get(z,0,0)==0.0,     "zeros");     pass("ml_zeros");
    o = ml_ones(2,2);  assert(ml_get(o,1,1)==1.0,     "ones");      pass("ml_ones");
    id = ml_identity(3);
    assert(ml_get(id,0,0)==1.0 && ml_get(id,0,1)==0.0,"identity"); pass("ml_identity");
    sh = ml_shape(z); assert(sh[0]==2.0&&sh[1]==3.0,  "shape");     pass("ml_shape");
    a = ml_ones(2,2); b = ml_ones(2,2);
    s = ml_add(a,b);   assert(ml_get(s,0,0)==2.0,     "add");       pass("ml_add");
    sc= ml_scale(a,5.0);assert(ml_get(sc,0,0)==5.0,   "scale");     pass("ml_scale");
    id2=ml_identity(2); m2=ml_new(2,2,3.0);
    res=ml_matmul(id2,m2); assert(ml_get(res,0,0)==3.0,"matmul");   pass("ml_matmul");
    v1=[1.0,2.0,3.0]; v2=[4.0,5.0,6.0];
    assert(ml_dot(v1,v2)==32.0,                        "dot");       pass("ml_dot");
    assert(sigmoid(0.0)==0.5,                          "sigmoid");   pass("sigmoid(0)");
    assert(relu(-3.0)==0.0,                            "relu neg");  pass("relu neg");
    assert(relu(5.0)==5.0,                             "relu pos");  pass("relu pos");
    sm=softmax([1.0,2.0,3.0]); tot=ml_sum(sm);
    assert(tot>0.99&&tot<1.01,                         "softmax");   pass("softmax sum=1");
    w=ml_random_weights(3,4); wsh=ml_shape(w);
    assert(wsh[0]==3.0&&wsh[1]==4.0,                   "randw");     pass("random_weights");
    arr=[1.0,2.0,3.0,4.0,5.0];
    assert(ml_sum(arr)==15.0,                          "sum");       pass("ml_sum");
    assert(ml_mean(arr)==3.0,                          "mean");      pass("ml_mean");
    assert(ml_max_val(arr)==5.0,                       "max");       pass("ml_max_val");
    assert(ml_min(arr)==1.0,                           "min");       pass("ml_min");
    nm=ml_normalize(arr);
    assert(ml_min(nm)==0.0&&ml_max_val(nm)==1.0,       "norm");      pass("ml_normalize");
}

// ═══════════════════════════════════════════════════════════
// SECTION 23 — HIGHER-ORDER FUNCTIONS
// ═══════════════════════════════════════════════════════════
fn dbl(x)     { return x * 2; }
fn isEven(x)  { return x % 2 == 0; }
fn sumUp(a,b) { return a + b; }

fn test_higher_order() {
    section(23, "Higher-Order Functions");
    nums = [1,2,3,4,5];
    mp = map_fn(nums,"dbl");
    assert(mp[0]==2&&mp[4]==10,      "map");    pass("map_fn");
    fi = filter_fn(nums,"isEven");
    assert(len(fi)==2&&fi[0]==2,     "filter"); pass("filter_fn");
    rd = reduce_fn(nums,"sumUp",0);
    assert(rd==15,                   "reduce"); pass("reduce_fn");
    assert(any_fn(nums,"isEven"),    "any");    pass("any_fn true");
    assert(!all_fn(nums,"isEven"),   "all no"); pass("all_fn false");
    assert(all_fn([2,4,6],"isEven"),"all yes"); pass("all_fn true");
}

// ═══════════════════════════════════════════════════════════
// SECTION 24 — IMPORT
// ═══════════════════════════════════════════════════════════
fn test_import() {
    section(24, "Import");
    write_file("_imp_helper.fx","fn imp_add(a,b){");
    append_file("_imp_helper.fx","\n    return a+b;");
    append_file("_imp_helper.fx","\n}");
    import "_imp_helper.fx";
    assert(imp_add(10,20)==30,   "import fn");  pass("import .fx function");
    import "_imp_helper.fx";     // double import safe
    assert(imp_add(1,1)==2,      "double imp"); pass("double import safe");
    import "math";
    assert(abs(-5)==5,           "import stdlib"); pass("import stdlib module");
    write_file("_imp_helper.fx",""); // cleanup
}

// ═══════════════════════════════════════════════════════════
// SECTION 25 — DOP: DOTIONS
// ═══════════════════════════════════════════════════════════
dotion Counter2 {
    count: 0,
    step:  1,
    fn increment() { self.count += self.step; }
    fn reset()     { self.count = 0; }
    fn get()       { return self.count; }
    on "add" (n)   { self.count += n; }
    on "reset"     { self.count = 0; }
}

fn test_dotions() {
    section(25, "DOP — Dotions");
    c = Counter2{};
    assert(c.get()==0,     "init");    pass("dotion init");
    c.increment();
    c.increment();
    assert(c.get()==2,     "method");  pass("dotion method");
    c.reset();
    assert(c.get()==0,     "reset");   pass("dotion reset method");
    send(c,"add",10);
    tick(1);
    assert(c.get()==10,    "handler"); pass("dotion handler send/tick");
    send(c,"reset");
    tick(1);
    assert(c.get()==0,     "on reset"); pass("dotion on reset");
}

// ═══════════════════════════════════════════════════════════
// SECTION 26 — DOP: BROADCAST & TICK
// ═══════════════════════════════════════════════════════════
dotion Light {
    is_on: false,
    on "turn_on"  { self.is_on = true; }
    on "turn_off" { self.is_on = false; }
}

fn test_broadcast() {
    section(26, "DOP — Broadcast & Tick");
    l1 = Light{};
    l2 = Light{};
    l3 = Light{};
    broadcast("turn_on");
    tick(1);
    assert(l1.is_on&&l2.is_on&&l3.is_on, "broadcast on"); pass("broadcast turn_on");
    broadcast("turn_off");
    tick(1);
    assert(!l1.is_on&&!l2.is_on,         "broadcast off"); pass("broadcast turn_off");
    assert(tick_count()>=2,               "tick_count");    pass("tick_count");
}

// ═══════════════════════════════════════════════════════════
// SECTION 27 — DOP: INHERITANCE
// ═══════════════════════════════════════════════════════════
dotion Animal {
    hp:    100,
    alive: true,
    fn is_alive() { return self.alive; }
    on "damage" (amount) {
        self.hp -= amount;
        if(self.hp <= 0) { self.alive = false; self.hp = 0; }
    }
}

dotion Cat extends Animal {
    name3: "cat",
    fn speak() { return "Meow!"; }
}

fn test_inheritance() {
    section(27, "DOP — Inheritance");
    c2 = Cat{};
    assert(c2.hp==100,           "inherited field"); pass("inherited field");
    assert(c2.alive==true,       "inherited alive"); pass("inherited alive");
    assert(c2.is_alive()==true,  "inherited method"); pass("inherited method");
    assert(c2.speak()=="Meow!",  "own method");       pass("own method");
    send(c2,"damage",30);
    tick(1);
    assert(c2.hp==70,            "inherited handler"); pass("inherited handler");
    send(c2,"damage",80);
    tick(1);
    assert(c2.hp==0,             "hp floored");   pass("hp floored at 0");
    assert(!c2.alive,            "died");          pass("alive=false after death");
}

// ═══════════════════════════════════════════════════════════
// SECTION 28 — DOP: TAGS & BROADCAST_TO
// ═══════════════════════════════════════════════════════════
dotion Enemy2 {
    hp2: 50,
    on "hit" (dmg) { self.hp2 -= dmg; }
} tags: ["enemy"]

dotion Ally2 {
    hp2: 100,
    on "hit" (dmg) { self.hp2 -= dmg; }
} tags: ["ally"]

fn test_tags() {
    section(28, "DOP — Tags & broadcast_to");
    e1 = Enemy2{};
    e2 = Enemy2{};
    a1 = Ally2{};
    broadcast_to("enemy","hit",10);
    tick(1);
    assert(e1.hp2==40,   "enemy1 hit");  pass("broadcast_to enemy 1");
    assert(e2.hp2==40,   "enemy2 hit");  pass("broadcast_to enemy 2");
    assert(a1.hp2==100,  "ally safe");   pass("ally not hit");
}

// ═══════════════════════════════════════════════════════════
// SECTION 29 — DOP: CLONE & QUERIES
// ═══════════════════════════════════════════════════════════
dotion Sprite2 { x2:0, y2:0, kind:"sprite" }

fn test_dop_queries() {
    section(29, "DOP — Clone & Queries");
    sp = Sprite2{};
    sp.x2 = 10; sp.y2 = 20;
    cp = clone(sp);
    assert(cp.x2==10&&cp.y2==20,    "clone fields");    pass("clone copies fields");
    cp.x2 = 99;
    assert(sp.x2==10,               "clone isolated");  pass("clone is independent");
    all = dotion_list();
    assert(len(all)>=1,             "dotion_list");     pass("dotion_list");
    cnt = dotion_count();
    assert(cnt>=1,                  "dotion_count");    pass("dotion_count");
}

// ═══════════════════════════════════════════════════════════
// SECTION 30 — ERROR REPORTING
// ═══════════════════════════════════════════════════════════
fn test_errors() {
    section(30, "Error Handling");
    err_msg = "";
    try {
        assert(false,"test error");
    } catch(e) {
        err_msg = e;
    }
    assert(str_len(err_msg)>0, "error has msg"); pass("error message captured");
    // Multiple independent try/catch blocks
    c1 = false; c2 = false;
    try { assert(false,"e1"); } catch(e) { c1 = true; }
    try { assert(false,"e2"); } catch(e) { c2 = true; }
    assert(c1 && c2, "multiple try"); pass("multiple independent try/catch");
}

// ═══════════════════════════════════════════════════════════
// MAIN
// ═══════════════════════════════════════════════════════════
start {
    out("");
    out("════════════════════════════════════════════════════");
    out("   FLUXIS v9.0 — Master Test Suite");
    out("════════════════════════════════════════════════════");

    test_primitives();
    test_arithmetic();
    test_logic();
    test_strings();
    test_interpolation();
    test_arrays();
    test_range();
    test_maps();
    test_control();
    test_functions();
    test_closures();
    test_defaults();
    test_in_operator();
    test_optional_null();
    test_try_catch();
    test_match();
    test_structs();
    test_enums();
    test_types();
    test_math();
    test_io();
    test_ml();
    test_higher_order();
    test_import();
    test_dotions();
    test_broadcast();
    test_inheritance();
    test_tags();
    test_dop_queries();
    test_errors();

    out("");
    out("════════════════════════════════════════════════════");
    out("   ALL TESTS PASSED — FLUXIS v9.0 CERTIFIED ✓");
    out("════════════════════════════════════════════════════");
    out("");
}

