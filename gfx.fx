import "gfx";

start {
    // Create a terminal canvas (width x height)
    gfx_canvas(40, 20);

    // Drawing primitives
    gfx_fill_rect(0, 0, 40, 20, ".");    // fill background
    gfx_rect(0, 0, 40, 20, "#");         // draw border
    gfx_line(0, 0, 39, 19, "-");        // diagonal line
    gfx_circle(20, 10, 8, "*");       // circle
    gfx_fill_rect(5, 5, 10, 5, " ");    // filled rect

    // Text
    gfx_text(10, 10, "FLUXIS v5");

    // Individual pixel
    gfx_pixel(15, 8, "@");

    // Render to terminal
    gfx_render();

    // PPM image export
    gfx_image("output.ppm");

    // Clear canvas
    gfx_clear();
    gfx_reset();

    // Color fill (for PPM)
    gfx_set_pixel(x, y, r, g, b);
}
