enum Color { Red, Green, Blue }
start {
  c = Color::Green;
  match c {
    Color::Red   => { out("red"); }
    Color::Green => { out("green"); }
    _            => { out("other"); }
  }
}
