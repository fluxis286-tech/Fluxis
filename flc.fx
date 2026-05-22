// ============================================================
// FLUXIS DOP FEATURE TEST — fl_dop_test.fx
// Tests Dotion-Oriented Programming:
//   dotions, fields, methods, handlers, send, broadcast,
//   tick system, actors, tags, tick_priority, inheritance
// ============================================================

fn pass(name) {
    out(format("  ✓  {}", name));
}

fn fail(name) {
    out(format("  ✗  FAIL: {}", name));
}

// ────────────────────────────────────────────────────────────
// 1. BASIC DOTION — fields and instantiation
// ────────────────────────────────────────────────────────────
dotion Counter {
    count: 0,
    name: "counter",
}

fn test_dotion_fields() {
    out("[ 1 ] Dotion Fields");

    c = Counter{};
    assert(c.count == 0, "dotion default field num");  pass("dotion default field num");
    assert(c.name == "counter", "dotion default field str"); pass("dotion default field str");
}

// ────────────────────────────────────────────────────────────
// 2. DOTION METHODS
// ────────────────────────────────────────────────────────────
dotion Box {
    width:  10,
    height: 5,

    fn area() {
        return self.width * self.height;
    }

    fn scale(factor) {
        self.width  = self.width  * factor;
        self.height = self.height * factor;
    }

    fn describe() {
        return format("{}x{}", self.width, self.height);
    }
}

fn test_dotion_methods() {
    out("[ 2 ] Dotion Methods");

    b = Box{};
    assert(b.area() == 50, "method return value"); pass("method return value");

    b.scale(2);
    assert(b.width == 20,  "method mutates self.width");  pass("method mutates self.width");
    assert(b.height == 10, "method mutates self.height"); pass("method mutates self.height");
    assert(b.area() == 200, "method after mutation"); pass("method after mutation");

    desc = b.describe();
    assert(contains(desc, "20"), "method returns string"); pass("method returns string");
}

// ────────────────────────────────────────────────────────────
// 3. DOTION HANDLERS (messaging)
// ────────────────────────────────────────────────────────────
dotion Lamp {
    is_on: false,
    brightness: 0,

    on "turn_on" {
        self.is_on = true;
        self.brightness = 100;
    }

    on "turn_off" {
        self.is_on = false;
        self.brightness = 0;
    }

    on "set_brightness" (val) {
        self.brightness = val;
    }
}

fn test_dotion_handlers() {
    out("[ 3 ] Dotion Handlers");

    lamp = Lamp{};
    assert(lamp.is_on == false, "initial state off"); pass("initial state off");

    send(lamp, "turn_on");
    tick(1);
    assert(lamp.is_on == true, "handler turn_on"); pass("handler turn_on");
    assert(lamp.brightness == 100, "handler sets brightness"); pass("handler sets brightness");

    send(lamp, "set_brightness", 50);
    tick(1);
    assert(lamp.brightness == 50, "handler with param"); pass("handler with param");

    send(lamp, "turn_off");
    tick(1);
    assert(lamp.is_on == false, "handler turn_off"); pass("handler turn_off");
    assert(lamp.brightness == 0, "handler resets brightness"); pass("handler resets brightness");
}

// ────────────────────────────────────────────────────────────
// 4. BROADCAST
// ────────────────────────────────────────────────────────────
dotion Node {
    active: false,
    id: 0,

    on "activate" {
        self.active = true;
    }

    on "deactivate" {
        self.active = false;
    }
}

fn test_broadcast() {
    out("[ 4 ] Broadcast");

    n1 = Node{};
    n2 = Node{};
    n3 = Node{};

    broadcast("activate");
    tick(1);

    assert(n1.active == true, "broadcast reaches n1"); pass("broadcast reaches n1");
    assert(n2.active == true, "broadcast reaches n2"); pass("broadcast reaches n2");
    assert(n3.active == true, "broadcast reaches n3"); pass("broadcast reaches n3");

    broadcast("deactivate");
    tick(1);

    assert(n1.active == false, "broadcast deactivate n1"); pass("broadcast deactivate n1");
    assert(n2.active == false, "broadcast deactivate n2"); pass("broadcast deactivate n2");
}

// ────────────────────────────────────────────────────────────
// 5. TICK SYSTEM
// ────────────────────────────────────────────────────────────
dotion Timer {
    elapsed: 0,

    on "tick_update" {
        self.elapsed += 1;
    }
}

fn test_tick() {
    out("[ 5 ] Tick System");

    t = Timer{};

    assert(t.elapsed == 0, "before tick"); pass("before tick");

    // Send 3 messages then tick — handlers fire on tick
    send(t, "tick_update");
    send(t, "tick_update");
    send(t, "tick_update");
    tick(1);
    assert(t.elapsed == 3, "after 3 ticks"); pass("after 3 ticks");

    send(t, "tick_update");
    send(t, "tick_update");
    tick(1);
    assert(t.elapsed == 5, "after 5 ticks total"); pass("after 5 ticks total");

    assert(tick_count() >= 2, "tick_count"); pass("tick_count");
}

// ────────────────────────────────────────────────────────────
// 6. SELF FIELD ACCESS & MUTATION
// ────────────────────────────────────────────────────────────
dotion Wallet {
    balance: 100,
    owner:   "Dipanshu",

    fn deposit(amount) {
        self.balance += amount;
    }

    fn withdraw(amount) {
        if(self.balance >= amount) {
            self.balance -= amount;
            return true;
        }
        return false;
    }

    fn get_balance() {
        return self.balance;
    }
}

fn test_self_mutation() {
    out("[ 6 ] Self Field Access & Mutation");

    w = Wallet{};
    assert(w.get_balance() == 100, "initial balance"); pass("initial balance");

    w.deposit(50);
    assert(w.get_balance() == 150, "after deposit"); pass("after deposit");

    ok = w.withdraw(30);
    assert(ok == true, "withdraw success"); pass("withdraw success");
    assert(w.get_balance() == 120, "balance after withdraw"); pass("balance after withdraw");

    fail_ok = w.withdraw(999);
    assert(fail_ok == false, "withdraw insufficient"); pass("withdraw insufficient");
    assert(w.get_balance() == 120, "balance unchanged after failed withdraw"); pass("balance unchanged after failed withdraw");
}

// ────────────────────────────────────────────────────────────
// 7. DOTION INHERITANCE (extends)
// ────────────────────────────────────────────────────────────
dotion Animal {
    name:  "animal",
    alive: true,
    hp:    100,

    fn is_alive() {
        return self.alive;
    }

    on "damage" (amount) {
        self.hp -= amount;
        if(self.hp <= 0) {
            self.alive = false;
            self.hp    = 0;
        }
    }
}

dotion Dog extends Animal {
    name:  "dog",
    breed: "labrador",

    fn speak() {
        return "Woof!";
    }
}

fn test_inheritance() {
    out("[ 7 ] Dotion Inheritance");

    d = Dog{};
    assert(d.name == "dog",       "overridden field");   pass("overridden field");
    assert(d.breed == "labrador", "own field");           pass("own field");
    assert(d.hp == 100,           "inherited field hp");  pass("inherited field hp");
    assert(d.alive == true,       "inherited field alive"); pass("inherited field alive");

    assert(d.is_alive() == true, "inherited method"); pass("inherited method");
    assert(d.speak() == "Woof!", "own method");        pass("own method");

    send(d, "damage", 30);
    tick(1);
    assert(d.hp == 70, "inherited handler"); pass("inherited handler");
    assert(d.alive == true, "still alive"); pass("still alive");

    send(d, "damage", 80);
    tick(1);
    assert(d.hp == 0,       "hp floored at 0");  pass("hp floored at 0");
    assert(d.alive == false, "dead after damage"); pass("dead after damage");
    assert(d.is_alive() == false, "is_alive false"); pass("is_alive false");
}

// ────────────────────────────────────────────────────────────
// 8. TAGS & broadcast_to
// ────────────────────────────────────────────────────────────
dotion Enemy {
    hp:    50,
    alive: true,

    on "hit" (dmg) {
        self.hp -= dmg;
        if(self.hp <= 0) { self.alive = false; }
    }
} tags: ["enemy", "ground"]

dotion Ally {
    hp: 100,

    on "hit" (dmg) {
        self.hp -= dmg;
    }
} tags: ["ally"]

fn test_tags() {
    out("[ 8 ] Tags & broadcast_to");

    e1 = Enemy{};
    e2 = Enemy{};
    a1 = Ally{};

    // hit only enemies
    broadcast_to("enemy", "hit", 20);
    tick(1);

    assert(e1.hp == 30, "enemy 1 hit"); pass("enemy 1 hit");
    assert(e2.hp == 30, "enemy 2 hit"); pass("enemy 2 hit");
    assert(a1.hp == 100, "ally not hit by broadcast_to enemy"); pass("ally not hit by broadcast_to enemy");
}

// ────────────────────────────────────────────────────────────
// 9. TICK PRIORITY
// ────────────────────────────────────────────────────────────
dotion First {
    order: [],

    on "record" {
        self.order = push(self.order, "first");
    }
} tick_priority: 0

dotion Second {
    order: [],

    on "record" {
        self.order = push(self.order, "second");
    }
} tick_priority: 10

fn test_tick_priority() {
    out("[ 9 ] Tick Priority");

    f = First{};
    s = Second{};

    send(f, "record");
    send(s, "record");
    tick(1);

    // First (priority 0) processes before Second (priority 10)
    assert(f.order[0] == "first",  "first processed"); pass("first processed");
    assert(s.order[0] == "second", "second processed"); pass("second processed");
}

// ────────────────────────────────────────────────────────────
// 10. ACTORS (brain system)
// ────────────────────────────────────────────────────────────
actor Predator {
    fn decide(target) {
        if(target.hp > 0) {
            send_self("hunt");
        }
    }
}

dotion Hunter {
    hp:    100,
    kills: 0,

    on "hunt" {
        self.kills += 1;
    }
} with Predator

fn test_actors() {
    out("[ 10 ] Actors (Brain System)");

    h = Hunter{};
    assert(h.kills == 0, "initial kills"); pass("initial kills");

    tick(3);
    assert(h.kills == 3, "actor hunts each tick"); pass("actor hunts each tick");
}

// ────────────────────────────────────────────────────────────
// 11. DOTION LIST & QUERIES
// ────────────────────────────────────────────────────────────
dotion Soldier {
    team:  "blue",
    alive: true,
    hp:    100,
}

fn test_dotion_queries() {
    out("[ 11 ] Dotion Queries");

    s1 = Soldier{};
    s2 = Soldier{};
    s3 = Soldier{};

    all = dotion_list();
    assert(len(all) >= 3, "dotion_list len"); pass("dotion_list len");

    count = dotion_count();
    assert(count >= 3, "dotion_count"); pass("dotion_count");

    blue = dotion_where("team", "blue");
    assert(len(blue) >= 3, "dotion_where"); pass("dotion_where");
}

// ────────────────────────────────────────────────────────────
// 12. CLONE
// ────────────────────────────────────────────────────────────
dotion Sprite {
    x:    0,
    y:    0,
    kind: "player",
}

fn test_clone() {
    out("[ 12 ] Clone");

    original = Sprite{};
    original.x = 10;
    original.y = 20;

    copy = clone(original);
    assert(copy.x == 10,      "clone copies x");    pass("clone copies x");
    assert(copy.y == 20,      "clone copies y");    pass("clone copies y");
    assert(copy.kind == "player", "clone copies kind"); pass("clone copies kind");

    // Mutate copy — original should not change
    copy.x = 99;
    assert(original.x == 10, "original unaffected"); pass("original unaffected");
    assert(copy.x == 99,      "copy mutated");        pass("copy mutated");
}

// ────────────────────────────────────────────────────────────
// MAIN
// ────────────────────────────────────────────────────────────
start {
    out("");
    out("====================================================");
    out("  FLUXIS DOP Feature Test");
    out("====================================================");
    out("");

    test_dotion_fields();    out("");
    test_dotion_methods();   out("");
    test_dotion_handlers();  out("");
    test_broadcast();        out("");
    test_tick();             out("");
    test_self_mutation();    out("");
    test_inheritance();      out("");
    test_tags();             out("");
    test_tick_priority();    out("");
    test_actors();           out("");
    test_dotion_queries();   out("");
    test_clone();            out("");

    out("====================================================");
    out("  ALL DOP TESTS PASSED");
    out("====================================================");
    out("");
}

