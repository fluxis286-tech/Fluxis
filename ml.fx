import "ml";

start {
    // XOR training data — real floats
    inputs  = [[0.0,0.0],[0.0,1.0],[1.0,0.0],[1.0,1.0]];
    targets = [0.0, 1.0, 1.0, 0.0];

    // Xavier-initialized weights
    w1 = ml_random_weights(4, 2);   // 4×2  hidden layer
    b1 = [0.0, 0.0, 0.0, 0.0];     // hidden biases
    w2 = ml_random_weights(1, 4);   // 1×4  output layer
    b2 = [0.0];                    // output bias

    lr = 0.5;   // learning rate

    out("XOR Neural Net — 2→4→1 with real float backprop");
    out("Epochs: 5000 | LR: " + lr);
    out("");

    for(epoch=1; epoch<=5000; epoch++;){
        total_loss = 0.0;

        for(i=0; i<4; i++;){
            // —— Forward pass ————————————————
            inp    = inputs[i];

            z1     = ml_layer_forward(inp, w1, b1);   // raw hidden [4]
            a1     = sigmoid_arr(z1);                 // activated  [4]

            z2     = ml_layer_forward(a1, w2, b2);    // raw output [1]
            a2     = sigmoid_arr(z2);                 // activated  [1]

            // —— Loss (MSE) ————————————————
            pred   = a2[0];
            target = targets[i];
            err    = pred - target;
            total_loss = total_loss + err * err;

            // —— Backward pass ————————————————
            // Output delta = error × sigmoid'(pred)
            sig2   = sigmoid_deriv_from_output(pred);
            delta2 = [err * sig2];

            // Gradient for W2: outer product
            grad_w2 = ml_outer(delta2, a1);

            // Hidden delta = W2^T·delta2 × sigmoid'(a1)
            back1  = ml_mat_T_vec(w2, delta2);
            sig1   = sigmoid_deriv_arr(a1);
            delta1 = ml_vec_mul(back1, sig1);

            // Gradient for W1: outer product
            grad_w1 = ml_outer(delta1, inp);

            // —— Update weights ————————————————
            w2 = ml_grad_desc_step(w2, grad_w2, lr);
            b2 = ml_bias_update(b2, delta2, lr);
            w1 = ml_grad_desc_step(w1, grad_w1, lr);
            b1 = ml_bias_update(b1, delta1, lr);
        }

        if(epoch % 100 == 0){
            out("Epoch " + epoch + " | loss: " + total_loss);
        }
    }

    // —— Final predictions ————————————————
    out("");
    out("—— XOR Final Predictions ——");
    out("Values close to 0.0 or 1.0 = good");
    out("");

    for(i=0; i<4; i++;){
        inp    = inputs[i];
        a1     = sigmoid_arr(ml_layer_forward(inp, w1, b1));
        a2     = sigmoid_arr(ml_layer_forward(a1,  w2, b2));
        pred   = a2[0];
        target = targets[i];
        out("Input: " + inp + "  →  pred: " + pred + "   target: " + target);
    }

    out("");
    out("✓ Loss should approach 0.0 after training.");
}
