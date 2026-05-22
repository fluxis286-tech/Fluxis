start{
ai_use("ollama");
    assert(ai_get_url() == "http://localhost:11434", "ai_use sets url");
    

    ai_set_model("qwen2.5:3b");
    assert(ai_get_model() == "qwen2.5:3b", "ai_set_model");
                                               
    // You can also set manually:
    i=1;
    // ai_set_url("http://localhost:11434");
    // ai_set_model("qwen2.5:3b");
    while(i>0){
    out("enter what you want to ask");
    a=in();
    if(a=="exit")
    {
    i=0;
    }
    else{
    out("output");
    r=ai_ask(a);
    out(r);
    }
    }
    }
