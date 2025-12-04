class Point {
  constructor(x, y) { this.x = x; this.y = y; }
  add(o) { return new Point(this.x + o.x, this.y + o.y); }
  toString() { return `(${this.x}, ${this.y})`; }
  static origin() { return new Point(0, 0); }
}
const p = new Point(1, 2).add(new Point(3, 4));
console.log(p.x, p.y, "" + p, String(Point.origin()));
console.log(p);
console.log(typeof Point, p instanceof Point);
