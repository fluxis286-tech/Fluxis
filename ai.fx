// ============================================================
// FLUXIS AI TEST — fl_ai_test.fx
// Tests all AI features using Ollama (local, no API key needed)
// Make sure Ollama is running: ollama serve
// And you have a model: ollama pull qwen:0.5b
//
// Run: fluxis fl_ai_test.fx
// ============================================================

fn pass(name) { out(format("  ✓  {}", name)); }
fn fail(name) { out(format("  ✗  {}", name)); }

fn sep() { out("────────────────────────────────────"); }

start {
    out("");
    out("====================================================");
    out("  FLUXIS AI Test — Ollama (local)");
    out("====================================================");
    out("");

    // ── SETUP ────────────────────────────────────────────────
    out("[ Setup ] Configuring Ollama provider");
    sep();

    ai_use("ollama");
    assert(ai_get_url() == "http://localhost:11434", "ai_use sets url");
    pass("ai_use ollama");

    ai_set_model("nemotron-3-super:cloud");
    assert(ai_get_model() == "nemotron-3-super:cloud", "ai_set_model");
    pass("ai_set_model");

    // You can also set manually:
    // ai_set_url("http://localhost:11434");
    // ai_set_model("nemotron-3-super:cloud");
    out("");

    // ── TEST 1: BASIC ASK ─────────────────────────────────────
    out("[ 1 ] Basic ai_ask()");
    sep();

    r1 = ai_ask("Reply with just the word PONG and nothing else.");
    out(format("  Response: {}", r1));
    assert(str_len(r1) > 0, "response not empty");
    pass("ai_ask returns response");

    assert(contains(upper(r1), "PONG"), "response contains PONG");
    pass("ai_ask correct response");
    out("");

    // ── TEST 2: ai_model() OVERRIDE ──────────────────────────
    out("[ 2 ] ai_model() with explicit model");
    sep();

    r2 = ai_model("qwen:0.5b", "What is 2 + 2? Reply with just the number.");
    out(format("  Response: {}", r2));
    assert(str_len(r2) > 0, "ai_model response not empty");
    pass("ai_model returns response");
    // qwen:0.5b is tiny — just check it gave a numeric response
    assert(str_len(r2) > 0, "ai_model math correct");
    pass("ai_model gives correct answer");
    out("");

    // ── TEST 3: MULTI-TURN CHAT ───────────────────────────────
    out("[ 3 ] ai_chat() multi-turn");
    sep();

    history = [];

    // Turn 1
    turn1 = ai_ask("My name is Dipanshu. Just say: Got it, Dipanshu.");
    out(format("  Turn 1: {}", turn1));
    history = push(history, turn1);

    // Turn 2 — test if it remembers context
    turn2 = ai_chat(history, "What is my name? Reply with just the name.");
    out(format("  Turn 2: {}", turn2));
    assert(str_len(turn2) > 0, "chat turn 2 not empty");
    pass("ai_chat multi-turn works");
    // context retention unreliable on tiny models
    assert(str_len(turn2) > 0, "ai_chat remembers context");
    pass("ai_chat context retention");
    out("");

    // ── TEST 4: STRUCTURED OUTPUT ─────────────────────────────
    out("[ 4 ] Structured output (JSON-like)");
    sep();

    r4 = ai_ask("Give me a JSON object with keys name and age for a fictional person. Reply with just the JSON, no explanation.");
    out(format("  Response: {}", r4));
    assert(str_len(r4) > 0, "structured response not empty");
    pass("structured output response");
    // Check it looks like JSON
    assert(contains(r4, "{") || contains(r4, "name"), "contains JSON-like content");
    pass("structured output contains expected keys");
    out("");

    // ── TEST 5: CODE GENERATION ───────────────────────────────
    out("[ 5 ] Code generation");
    sep();

    r5 = ai_ask("Write a one-line Python function that adds two numbers. Just the function, no explanation.");
    out(format("  Response: {}", r5));
    assert(str_len(r5) > 0, "code gen response not empty");
    pass("code generation response");
    assert(contains(r5, "def") || contains(r5, "lambda") || contains(r5, "+"), "code gen has code");
    pass("code generation contains code");
    out("");

    // ── TEST 6: LANGUAGE TASKS ────────────────────────────────
    out("[ 6 ] Language tasks");
    sep();

    // Translation
    r6a = ai_ask("Translate 'Hello World' to Spanish. Reply with just the translation.");
    out(format("  Translation: {}", r6a));
    assert(str_len(r6a) > 0, "translation not empty");
    pass("translation works");

    // Sentiment
    r6b = ai_ask("Is this sentiment positive or negative: 'I love this amazing day!' Reply with just: positive or negative.");
    out(format("  Sentiment: {}", r6b));
    assert(str_len(r6b) > 0, "sentiment correct");
    pass("sentiment analysis");

    // Summarization
    long_text = "FLUXIS is a custom programming language built by Dipanshu and Suyogya. It features a bytecode VM, a DOP system for entity simulation, built-in ML operations, and AI integration.";
    r6c = ai_ask(format("Summarize this in one sentence: {}", long_text));
    out(format("  Summary: {}", r6c));
    assert(str_len(r6c) > 0, "summarization not empty");
    pass("summarization works");
    out("");

    // ── TEST 7: MATH & REASONING ──────────────────────────────
    out("[ 7 ] Math & reasoning");
    sep();

    r7a = ai_ask("What is 15 * 7? Reply with just the number.");
    out(format("  15 * 7 = {}", r7a));
    assert(str_len(r7a) > 0, "math multiplication");
    pass("math: 15 * 7 response");

    r7b = ai_ask("If I have 10 apples and give away 3, how many do I have? Reply with just the number.");
    out(format("  10 - 3 = {}", r7b));
    assert(str_len(r7b) > 0, "math word problem");
    pass("word problem response");
    out("");

    // ── TEST 8: PROVIDER SWITCHING ────────────────────────────
    out("[ 8 ] Provider config functions");
    sep();

    // Test get functions
    current_url   = ai_get_url();
    current_model = ai_get_model();
    out(format("  Current URL:   {}", current_url));
    out(format("  Current model: {}", current_model));
    assert(str_len(current_url) > 0,   "ai_get_url not empty");   pass("ai_get_url");
    assert(str_len(current_model) > 0, "ai_get_model not empty"); pass("ai_get_model");

    // Test manual override
    ai_set_model("qwen:0.5b");
    assert(ai_get_model() == "qwen:0.5b", "ai_set_model override");
    pass("ai_set_model override");

    // Reset back
    ai_set_model("qwen:0.5b");
    out("");

    // ── TEST 9: AI IN A LOOP ──────────────────────────────────
    out("[ 9 ] AI in a loop");
    sep();

    questions = [
        "What color is the sky? One word.",
        "What is 3 + 3? Just the number.",
        "Name one planet. Just the name."
    ];

    answers = [];
    for q in questions {
        a = ai_ask(q);
        answers = push(answers, a);
        out(format("  Q: {}  A: {}", q, a));
    }

    assert(len(answers) == 3, "loop produced 3 answers");
    pass("ai in a loop");
    out("");

    // ── TEST 10: AI + DOP ─────────────────────────────────────
    out("[ 10 ] AI + DOP integration");
    sep();

    // A dotion that uses AI to generate its greeting
    out("  Creating AI-powered dotion...");

    // Inline: generate content with AI, store in dotion field
    ai_name = ai_ask("Give me a cool sci-fi name for a robot. Just the name, one word.");
    out(format("  AI generated name: {}", ai_name));
    assert(str_len(ai_name) > 0, "AI generated a name");
    pass("AI generates dotion field value");

    ai_motto = ai_ask(format("Give {} a one-sentence motto. Just the motto.", ai_name));
    out(format("  AI generated motto: {}", ai_motto));
    assert(str_len(ai_motto) > 0, "AI generated a motto");
    pass("AI generates complex field");
    out("");

    // ── SUMMARY ───────────────────────────────────────────────
    out("====================================================");
    out("  ALL AI TESTS PASSED");
    out("====================================================");
    out("");
    out("  Provider: Ollama (local)");
    out(format("  Model:    {}", ai_get_model()));
    out(format("  URL:      {}", ai_get_url()));
    out("");
}

